//! V2 native Windows shell: no .NET, Chromium, WebView, or local HTTP server.

use chrono::DateTime;
use r5_battery_estimator::{ProbeResult, battery::transport::R5HidTransport, probe_once};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs,
    mem::{MaybeUninit, size_of},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows::{
    Win32::{
        Foundation::{COLORREF, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreatePen,
            CreateRoundRectRgn, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER,
            DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, Ellipse, EndPaint,
            FW_BOLD, FW_NORMAL, FillRect, GetStockObject, HOLLOW_BRUSH, InvalidateRect, LineTo,
            MoveToEx, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, RoundRect, SelectObject,
            SetBkMode, SetTextColor, SetWindowRgn, TRANSPARENT,
        },
        Graphics::GdiPlus::{
            GdipCreateBitmapFromFile, GdipCreateFromHDC, GdipCreateHICONFromBitmap, GdipCreatePen1,
            GdipDeleteGraphics, GdipDeletePen, GdipDisposeImage, GdipDrawArcI, GdipDrawImageRectI,
            GdipDrawLinesI, GdipSetPenEndCap, GdipSetPenStartCap, GdipSetSmoothingMode,
            GdiplusStartup, GdiplusStartupInput, LineCapRound, Point, SmoothingModeAntiAlias,
            UnitPixel,
        },
        System::LibraryLoader::GetModuleHandleW,
        System::Threading::CreateMutexW,
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreatePopupMenu,
                CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW,
                GetCursorPos, GetMessageW, HICON, HTCAPTION, IDC_ARROW, IDI_APPLICATION,
                LoadCursorW, LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PostQuitMessage,
                RegisterClassW, SW_HIDE, SW_SHOW, SendMessageW, SetForegroundWindow, SetTimer,
                ShowWindow, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CLOSE,
                WM_COMMAND, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT,
                WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_POPUP, WS_VISIBLE,
            },
        },
    },
    core::{PCWSTR, w},
};

const CLASS: windows::core::PCWSTR = w!("R5BatteryEstimatorNativeV2");
const TRAY_MESSAGE: u32 = WM_APP + 17;
const WIDTH: i32 = 1140;
const HEIGHT: i32 = 950;
const CHROME_HEIGHT: i32 = 48;
const REFRESH: Rect = Rect::new(852, 872, 228, 60);
const MINIMIZE: Rect = Rect::new(1028, 0, 56, 48);
const CLOSE: Rect = Rect::new(1084, 0, 56, 48);
static BATTERY: OnceLock<Mutex<ProbeResult>> = OnceLock::new();
static HISTORY: OnceLock<Mutex<Vec<Sample>>> = OnceLock::new();

struct BrandImages {
    shark: *mut windows::Win32::Graphics::GdiPlus::GpImage,
}
thread_local! { static BRAND_IMAGES: RefCell<Option<BrandImages>> = const { RefCell::new(None) }; }

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl Rect {
    const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Win32 COLORREF is BGR in memory. Accept normal CSS-style RRGGBB at every call site.
const fn rgb(hex: u32) -> COLORREF {
    COLORREF(((hex & 0x0000ff) << 16) | (hex & 0x00ff00) | ((hex & 0xff0000) >> 16))
}

#[derive(Clone, Serialize, Deserialize)]
struct Sample {
    at: u64,
    percent: u8,
    charging: bool,
}

#[derive(Deserialize)]
struct LegacySample {
    #[serde(rename = "At")]
    at: String,
    #[serde(rename = "Percent")]
    percent: u8,
    #[serde(rename = "Charging")]
    charging: bool,
}

fn main() -> windows::core::Result<()> {
    unsafe {
        // GDI+ is used only by the static anti-aliased ring. It never runs in the HID worker.
        let mut gdiplus_token = 0usize;
        let startup = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let _ = GdiplusStartup(&mut gdiplus_token, &startup, std::ptr::null_mut());
        let _instance_guard = CreateMutexW(None, false, w!("Local\\R5BatteryEstimatorNativeV2"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        let initial = probe_once(&R5HidTransport::new());
        let state = BATTERY.get_or_init(|| Mutex::new(initial.clone()));
        HISTORY.get_or_init(|| Mutex::new(load_history()));
        record_sample(&initial);
        let state = state as &'static Mutex<ProbeResult>;
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(30));
                let result = probe_once(&R5HidTransport::new());
                *state.lock().expect("battery state poisoned") = result.clone();
                record_sample(&result);
            }
        });
        let instance = GetModuleHandleW(None)?;
        let cursor = LoadCursorW(None, IDC_ARROW)?;
        let class = WNDCLASSW {
            hCursor: cursor,
            hIcon: native_icon(),
            hInstance: instance.into(),
            lpszClassName: CLASS,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS,
            w!("R5 Battery Estimator"),
            WS_POPUP | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIDTH,
            HEIGHT,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        // A deterministic Win10/11 fallback for the custom borderless shell.
        // Windows takes ownership of this region after SetWindowRgn succeeds.
        let region = CreateRoundRectRgn(0, 0, WIDTH, HEIGHT, 24, 24);
        let _ = SetWindowRgn(hwnd, Some(region), true);
        add_tray(hwnd)?;
        update_tray(hwnd);
        let _ = SetTimer(Some(hwnd), 1, 30_000, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let mut message = MaybeUninit::<MSG>::zeroed();
        while GetMessageW(message.as_mut_ptr(), None, 0, 0).into() {
            let message = message.assume_init();
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        TRAY_MESSAGE if lparam.0 as u32 == WM_LBUTTONUP => {
            let _ = ShowWindow(hwnd, SW_SHOW);
            LRESULT(0)
        }
        TRAY_MESSAGE if lparam.0 as u32 == WM_RBUTTONUP => {
            show_tray_menu(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => match (wparam.0 & 0xffff) as usize {
            1 => {
                let _ = ShowWindow(hwnd, SW_SHOW);
                LRESULT(0)
            }
            2 => {
                if let Some(state) = BATTERY.get() {
                    let result = probe_once(&R5HidTransport::new());
                    *state.lock().expect("battery state poisoned") = result.clone();
                    record_sample(&result);
                }
                update_tray(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            3 => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            _ => LRESULT(0),
        },
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            release_brand_images();
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xffff) as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i32;
            if CLOSE.contains(x, y) {
                let _ = ShowWindow(hwnd, SW_HIDE);
                release_brand_images();
            } else if MINIMIZE.contains(x, y) {
                let _ = ShowWindow(hwnd, SW_HIDE);
                release_brand_images();
            } else if REFRESH.contains(x, y) {
                if let Some(state) = BATTERY.get() {
                    let result = probe_once(&R5HidTransport::new());
                    *state.lock().expect("battery state poisoned") = result.clone();
                    record_sample(&result);
                }
                update_tray(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else if y < CHROME_HEIGHT {
                let _ = ReleaseCapture();
                let _ = SendMessageW(
                    hwnd,
                    WM_NCLBUTTONDOWN,
                    Some(WPARAM(HTCAPTION as usize)),
                    Some(LPARAM(0)),
                );
            }
            LRESULT(0)
        }
        WM_TIMER => {
            update_tray(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let background = CreateSolidBrush(rgb(0x0A1117));
            FillRect(dc, &paint.rcPaint, background);
            let _ = DeleteObject(background.into());
            let header = CreateSolidBrush(rgb(0x0D161D));
            FillRect(
                dc,
                &windows::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: WIDTH,
                    bottom: CHROME_HEIGHT,
                },
                header,
            );
            let _ = DeleteObject(header.into());
            SetBkMode(dc, TRANSPARENT);
            let (percent, status) =
                match BATTERY.get().and_then(|s| s.lock().ok()).map(|s| s.clone()) {
                    Some(ProbeResult::Ok { reading }) => (
                        Some(reading.percent),
                        if reading.charging {
                            "Cargando"
                        } else {
                            "No cargando"
                        },
                    ),
                    _ => (None, "Sin lectura válida"),
                };
            let samples = HISTORY
                .get()
                .and_then(|history| history.lock().ok())
                .map(|items| items.clone())
                .unwrap_or_default();
            let learned = learned_hours(&samples);
            draw(
                dc,
                "R5 Battery Estimator",
                60,
                78,
                700,
                70,
                42,
                0xF7F9FC,
                true,
            );
            draw_brand_icon(dc, 20, 10, 28);
            draw(
                dc,
                "R5 Battery Estimator",
                66,
                0,
                320,
                CHROME_HEIGHT,
                15,
                0xF7F9FC,
                false,
            );
            draw_center(
                dc, "—", MINIMIZE.x, 0, MINIMIZE.w, MINIMIZE.h, 18, 0xF7F9FC, false,
            );
            draw_center(dc, "×", CLOSE.x, 0, CLOSE.w, CLOSE.h, 22, 0xF7F9FC, false);
            draw(
                dc,
                "Se aprende con tu descarga real.",
                62,
                143,
                700,
                42,
                18,
                0xBED0E9,
                false,
            );
            draw_right(dc, "R5 ULTRA", 860, 84, 130, 45, 18, 0xFF5353, true);
            draw_mascot(dc, 1010, 62, 84);
            card(dc, 390, 265, 350, 230);
            card(dc, 765, 265, 340, 230);
            chart(dc, &samples);
            button(dc);
            ring(dc, 58, 198, 305, percent, status);
            clock_glyph(dc, 429, 320);
            draw(
                dc,
                "DURACIÓN DE CARGA COMPLETA",
                449,
                305,
                290,
                30,
                11,
                0xBED0E9,
                false,
            );
            draw(
                dc,
                &learned
                    .map(|hours| format!("{hours:.0} h"))
                    .unwrap_or_else(|| "Aprendiendo".to_owned()),
                420,
                365,
                290,
                62,
                31,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se aprende con tu descarga real.",
                420,
                445,
                290,
                30,
                11,
                0xBED0E9,
                false,
            );
            bars_glyph(dc, 802, 333);
            draw(
                dc,
                "AUTONOMÍA RESTANTE",
                826,
                305,
                270,
                30,
                11,
                0xBED0E9,
                false,
            );
            draw(
                dc,
                &percent
                    .map(|value| format!("{:.0} h", remaining_hours(learned, value)))
                    .unwrap_or_else(|| "—".to_owned()),
                795,
                365,
                270,
                70,
                48,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se recalcula cada 30 segundos.",
                795,
                445,
                280,
                30,
                11,
                0xBED0E9,
                false,
            );
            draw(
                dc,
                "La app nunca muestra una desconexión como 0%.",
                55,
                885,
                600,
                30,
                13,
                0xBED0E9,
                false,
            );
            // Hairline inside the rounded region keeps the shell intentionally rounded instead of full-bleed square.
            let hollow = GetStockObject(HOLLOW_BRUSH);
            let old_brush = SelectObject(dc, hollow);
            let root_border = CreatePen(PS_SOLID, 1, rgb(0x233440));
            let old_pen = SelectObject(dc, root_border.into());
            let _ = RoundRect(dc, 0, 0, WIDTH - 1, HEIGHT - 1, 24, 24);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            let _ = DeleteObject(root_border.into());
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_DESTROY => {
            release_brand_images();
            remove_tray(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = match CreatePopupMenu() {
        Ok(menu) => menu,
        Err(_) => return,
    };
    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Abrir panel"));
    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Actualizar perfil"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
    let _ = AppendMenuW(menu, MF_STRING, 3, w!("Cerrar programa"));
    let mut point = windows::Win32::Foundation::POINT::default();
    if GetCursorPos(&mut point).is_ok() {
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, Default::default(), point.x, point.y, None, hwnd, None);
    }
    let _ = DestroyMenu(menu);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw(
    dc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    size: i32,
    color: u32,
    bold: bool,
) {
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        if bold {
            FW_BOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(dc, font.into());
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, rgb(color));
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = windows::Win32::Foundation::RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    DrawTextW(
        dc,
        &mut wide,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(font.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_center(
    dc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    size: i32,
    color: u32,
    bold: bool,
) {
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        if bold {
            FW_BOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(dc, font.into());
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, rgb(color));
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = windows::Win32::Foundation::RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    DrawTextW(
        dc,
        &mut wide,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(font.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_right(
    dc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    size: i32,
    color: u32,
    bold: bool,
) {
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        if bold {
            FW_BOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let old = SelectObject(dc, font.into());
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, rgb(color));
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = windows::Win32::Foundation::RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    DrawTextW(
        dc,
        &mut wide,
        &mut rect,
        windows::Win32::Graphics::Gdi::DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(font.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn card(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, width: i32, height: i32) {
    let shadow = CreateSolidBrush(rgb(0x05090D));
    let old_shadow = SelectObject(dc, shadow.into());
    let _ = RoundRect(dc, x + 4, y + 5, x + width + 4, y + height + 5, 18, 18);
    SelectObject(dc, old_shadow);
    let _ = DeleteObject(shadow.into());
    let brush = CreateSolidBrush(rgb(0x101B23));
    let old = SelectObject(dc, brush.into());
    let _ = RoundRect(dc, x, y, x + width, y + height, 18, 18);
    SelectObject(dc, old);
    let _ = DeleteObject(brush.into());
    let hollow = GetStockObject(HOLLOW_BRUSH);
    let old_brush = SelectObject(dc, hollow);
    let border = CreatePen(PS_SOLID, 1, rgb(0x233440));
    let old_pen = SelectObject(dc, border.into());
    let _ = RoundRect(dc, x, y, x + width, y + height, 18, 18);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(border.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn chart(dc: windows::Win32::Graphics::Gdi::HDC, samples: &[Sample]) {
    card(dc, 38, 570, 1064, 280);
    // The tallest bar and heading share the same optical vertical centre.
    bars_glyph(dc, 92, 632);
    draw(
        dc,
        "HISTORIAL: HORA / PORCENTAJE",
        120,
        605,
        420,
        30,
        15,
        0xBED0E9,
        true,
    );
    draw_right(dc, "100%", 70, 653, 60, 25, 11, 0xBED0E9, false);
    draw_right(dc, "0%", 90, 787, 40, 25, 11, 0xBED0E9, false);
    draw(dc, "24 h", 145, 815, 80, 25, 11, 0xBED0E9, false);
    draw_right(dc, "ahora", 1005, 815, 63, 25, 11, 0xBED0E9, false);
    let grid = CreatePen(PS_SOLID, 1, rgb(0x334956));
    let old = SelectObject(dc, grid.into());
    for row in 0..4 {
        let y = 665 + row * 34;
        let _ = MoveToEx(dc, 145, y, None);
        let _ = LineTo(dc, 1068, y);
    }
    for col in 1..4 {
        let x = 145 + col * 231;
        let _ = MoveToEx(dc, x, 665, None);
        let _ = LineTo(dc, x, 799);
    }
    SelectObject(dc, old);
    let _ = DeleteObject(grid.into());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start = now.saturating_sub(24 * 60 * 60);
    let visible: Vec<_> = samples
        .iter()
        .filter(|sample| sample.at >= start && sample.at <= now)
        .collect();
    if visible.len() >= 2 {
        let points: Vec<Point> = visible.iter().map(|sample| Point {
            X: 145 + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0) as i32,
            Y: 799 - sample.percent as i32 * 134 / 100,
        }).collect();
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
            let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            // Same three-layer light treatment as the battery ring, restrained for a data series.
            draw_gp_lines(graphics, &points, 0x122FE189, 16.0);
            draw_gp_lines(graphics, &points, 0x382FE189, 8.0);
            draw_gp_lines(graphics, &points, 0xFF2FE189, 3.0);
            let _ = GdipDeleteGraphics(graphics);
        }
    } else if let Some(sample) = visible.first() {
        let x = 145
            + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0) as i32;
        let y = 799 - sample.percent as i32 * 134 / 100;
        let brush = CreateSolidBrush(rgb(0x2FE189));
        let old = SelectObject(dc, brush.into());
        let _ = Ellipse(dc, x - 3, y - 3, x + 4, y + 4);
        SelectObject(dc, old);
        let _ = DeleteObject(brush.into());
    } else {
        draw_center(
            dc,
            "Aún no hay historial",
            330,
            705,
            560,
            24,
            13,
            0xBED0E9,
            false,
        );
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn button(dc: windows::Win32::Graphics::Gdi::HDC) {
    let brush = CreateSolidBrush(rgb(0xE8373D));
    let old = SelectObject(dc, brush.into());
    let _ = RoundRect(
        dc,
        REFRESH.x,
        REFRESH.y,
        REFRESH.x + REFRESH.w,
        REFRESH.y + REFRESH.h,
        24,
        24,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(brush.into());
    draw_center(
        dc,
        "Actualizar ahora",
        REFRESH.x,
        REFRESH.y,
        REFRESH.w,
        REFRESH.h,
        15,
        0xFFFFFF,
        false,
    );
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_brand_icon(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, size: i32) {
    draw_brand_image(dc, x, y, size, size);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_mascot(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, size: i32) {
    draw_brand_image(dc, x, y, size, size);
}

fn asset_path(name: &str) -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("R5BatteryEstimator.Native")
        .join("Assets")
        .join(name);
    [
        exe_dir.as_ref().map(|dir| dir.join("Assets").join(name)),
        exe_dir.as_ref().map(|dir| dir.join(name)),
        Some(source),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn load_brand_image(name: &str) -> *mut windows::Win32::Graphics::GdiPlus::GpImage {
    let Some(path) = asset_path(name) else {
        return std::ptr::null_mut();
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut bitmap = std::ptr::null_mut();
    if GdipCreateBitmapFromFile(PCWSTR(wide.as_ptr()), &mut bitmap).0 == 0 {
        bitmap.cast()
    } else {
        std::ptr::null_mut()
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_brand_image(
    dc: windows::Win32::Graphics::Gdi::HDC,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    BRAND_IMAGES.with(|images| {
        let mut images = images.borrow_mut();
        if images.is_none() {
            *images = Some(BrandImages {
                shark: load_brand_image("shark-battery.png"),
            });
        }
        let image = images.as_ref().map(|assets| assets.shark).unwrap_or(std::ptr::null_mut());
        if image.is_null() {
            return;
        }
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
            let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            let _ = GdipDrawImageRectI(graphics, image, x, y, width, height);
            let _ = GdipDeleteGraphics(graphics);
        }
    });
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn release_brand_images() {
    BRAND_IMAGES.with(|images| {
        if let Some(images) = images.borrow_mut().take() {
            if !images.shark.is_null() { let _ = GdipDisposeImage(images.shark); }
        }
    });
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn native_icon() -> HICON {
    let fallback =
        || LoadIconW(None, IDI_APPLICATION).expect("Windows application icon must exist");
    let image = load_brand_image("shark-battery.png");
    if !image.is_null() {
        let mut icon = HICON::default();
        let status = GdipCreateHICONFromBitmap(image.cast(), &mut icon);
        let _ = GdipDisposeImage(image);
        if status.0 == 0 {
            return icon;
        }
    }
    fallback()
}

fn history_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("R5 Battery Estimator")
        .join("rust-v2-history.json")
}

fn load_history() -> Vec<Sample> {
    if let Some(history) = fs::read_to_string(history_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
    {
        return history;
    }
    let legacy_path = history_path().with_file_name("battery-history.json");
    let migrated = fs::read_to_string(legacy_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<LegacySample>>(&raw).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|sample| {
            DateTime::parse_from_rfc3339(&sample.at)
                .ok()
                .and_then(|at| u64::try_from(at.timestamp()).ok())
                .map(|at| Sample {
                    at,
                    percent: sample.percent,
                    charging: sample.charging,
                })
        })
        .collect::<Vec<_>>();
    migrated
}

fn record_sample(result: &ProbeResult) {
    let ProbeResult::Ok { reading } = result else {
        return;
    };
    let Some(history) = HISTORY.get() else { return };
    let mut items = history.lock().expect("history poisoned");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sample = Sample {
        at: now,
        percent: reading.percent,
        charging: reading.charging,
    };
    if let Some(last) = items
        .last_mut()
        .filter(|last| now.saturating_sub(last.at) < 15)
    {
        *last = sample;
    } else {
        items.push(sample);
    }
    let cutoff = now.saturating_sub(14 * 24 * 60 * 60);
    items.retain(|sample| sample.at >= cutoff);
    if let Some(parent) = history_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(&*items) {
        let _ = fs::write(history_path(), raw);
    }
}

fn learned_hours(samples: &[Sample]) -> Option<f64> {
    let start = samples
        .iter()
        .position(|sample| sample.percent >= 95 && !sample.charging)?;
    if !samples[start..]
        .iter()
        .any(|sample| sample.percent <= 5 && !sample.charging)
    {
        return None;
    }
    let mut elapsed = 0_u64;
    let mut dropped = 0_u64;
    for pair in samples[start..].windows(2) {
        let before = &pair[0];
        let after = &pair[1];
        let gap = after.at.saturating_sub(before.at);
        if before.charging
            || after.charging
            || gap == 0
            || gap > 600
            || after.percent > before.percent
        {
            continue;
        }
        elapsed += gap;
        dropped += (before.percent - after.percent) as u64;
    }
    if elapsed < 1800 || dropped < 3 {
        return None;
    }
    Some((100.0 / (dropped as f64 / (elapsed as f64 / 3600.0))).clamp(5.0, 1000.0))
}

/// A clearly provisional baseline keeps the live autonomy useful before a full
/// discharge has been observed. The adjacent full-charge card remains
/// `Aprendiendo`, so this is never presented as a learned calibration.
fn remaining_hours(learned: Option<f64>, percent: u8) -> f64 {
    learned.unwrap_or(200.0) * percent as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::{Sample, learned_hours};

    #[test]
    fn does_not_claim_learning_without_a_full_discharge() {
        let samples = [
            Sample {
                at: 0,
                percent: 100,
                charging: false,
            },
            Sample {
                at: 3_600,
                percent: 80,
                charging: false,
            },
        ];
        assert_eq!(learned_hours(&samples), None);
    }

    #[test]
    fn learns_only_from_a_valid_full_discharge() {
        let samples = [
            Sample {
                at: 0,
                percent: 100,
                charging: false,
            },
            Sample {
                at: 300,
                percent: 85,
                charging: false,
            },
            Sample {
                at: 600,
                percent: 70,
                charging: false,
            },
            Sample {
                at: 900,
                percent: 55,
                charging: false,
            },
            Sample {
                at: 1_200,
                percent: 40,
                charging: false,
            },
            Sample {
                at: 1_500,
                percent: 25,
                charging: false,
            },
            Sample {
                at: 1_800,
                percent: 10,
                charging: false,
            },
            Sample {
                at: 2_100,
                percent: 5,
                charging: false,
            },
        ];
        let hours = learned_hours(&samples).expect("valid cycle should learn");
        assert_eq!(
            hours, 5.0,
            "floor prevents implausibly short full-charge claims"
        );
    }

    #[test]
    fn ignores_long_gaps_and_charging_segments() {
        let samples = [
            Sample {
                at: 0,
                percent: 100,
                charging: false,
            },
            Sample {
                at: 3_600,
                percent: 90,
                charging: false,
            },
            Sample {
                at: 7_500,
                percent: 89,
                charging: false,
            },
            Sample {
                at: 7_530,
                percent: 88,
                charging: true,
            },
            Sample {
                at: 7_560,
                percent: 5,
                charging: false,
            },
        ];
        assert_eq!(learned_hours(&samples), None);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn ring(
    dc: windows::Win32::Graphics::Gdi::HDC,
    x: i32,
    y: i32,
    size: i32,
    percent: Option<u8>,
    status: &str,
) {
    // GDI's Arc produces square, stair-stepped caps. The static GDI+ layer is
    // antialiased, uses proper round caps and is invoked only for WM_PAINT.
    let left = x + 34;
    let top = y + 34;
    let diameter = size - 68;
    let center_x = x + size / 2;
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        draw_gp_arc(
            graphics, left, top, diameter, 0xff24323d, 22.0, -90.0, 359.95, true,
        );
        if let Some(value) = percent.filter(|value| *value > 0) {
            let sweep = value as f32 * 3.6;
            // Three green alpha layers create an emitted-light aura rather than a shadow.
            draw_gp_arc(
                graphics,
                left - 16,
                top - 16,
                diameter + 32,
                0x0a2fe189,
                52.0,
                -90.0,
                sweep,
                true,
            );
            draw_gp_arc(
                graphics,
                left - 10,
                top - 10,
                diameter + 20,
                0x1e2fe189,
                40.0,
                -90.0,
                sweep,
                true,
            );
            draw_gp_arc(
                graphics,
                left - 4,
                top - 4,
                diameter + 8,
                0x4f2fe189,
                28.0,
                -90.0,
                sweep,
                true,
            );
            draw_gp_arc(
                graphics,
                left,
                top,
                diameter,
                0xff2fe189,
                22.0,
                -90.0,
                sweep.min(359.95),
                value < 100,
            );
            if value == 100 {
                draw_gp_arc(
                    graphics, left, top, diameter, 0xff2fe189, 22.0, -90.0, 359.95, false,
                );
            }
        }
        let _ = GdipDeleteGraphics(graphics);
    }
    let value = percent
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "—%".to_owned());
    draw_center(dc, &value, x + 50, y + 98, 205, 62, 42, 0xF7F9FC, true);
    battery_glyph(dc, center_x, y + 178, percent.is_some());
    draw_center(dc, status, x + 48, y + 212, 210, 28, 15, 0xBED0E9, false);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_gp_arc(
    graphics: *mut windows::Win32::Graphics::GdiPlus::GpGraphics,
    x: i32,
    y: i32,
    d: i32,
    argb: u32,
    width: f32,
    start: f32,
    sweep: f32,
    rounded: bool,
) {
    let mut pen = std::ptr::null_mut();
    if GdipCreatePen1(argb, width, UnitPixel, &mut pen).0 == 0 {
        if rounded {
            let _ = GdipSetPenStartCap(pen, LineCapRound);
            let _ = GdipSetPenEndCap(pen, LineCapRound);
        }
        let _ = GdipDrawArcI(graphics, pen, x, y, d, d, start, sweep);
        let _ = GdipDeletePen(pen);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_gp_lines(
    graphics: *mut windows::Win32::Graphics::GdiPlus::GpGraphics,
    points: &[Point],
    argb: u32,
    width: f32,
) {
    let mut pen = std::ptr::null_mut();
    if GdipCreatePen1(argb, width, UnitPixel, &mut pen).0 == 0 {
        let _ = GdipSetPenStartCap(pen, LineCapRound);
        let _ = GdipSetPenEndCap(pen, LineCapRound);
        let _ = GdipDrawLinesI(graphics, pen, points.as_ptr(), points.len() as i32);
        let _ = GdipDeletePen(pen);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn battery_glyph(
    dc: windows::Win32::Graphics::Gdi::HDC,
    center_x: i32,
    top: i32,
    valid: bool,
) {
    let pen = CreatePen(PS_SOLID, 3, rgb(if valid { 0x2FE189 } else { 0x7890A4 }));
    let hollow = GetStockObject(HOLLOW_BRUSH);
    let old_brush = SelectObject(dc, hollow);
    let old = SelectObject(dc, pen.into());
    let _ = RoundRect(dc, center_x - 21, top, center_x + 19, top + 19, 3, 3);
    let _ = MoveToEx(dc, center_x + 20, top + 6, None);
    let _ = LineTo(dc, center_x + 25, top + 6);
    let _ = LineTo(dc, center_x + 25, top + 14);
    let _ = LineTo(dc, center_x + 20, top + 14);
    SelectObject(dc, old);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(pen.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn clock_glyph(dc: windows::Win32::Graphics::Gdi::HDC, center_x: i32, center_y: i32) {
    let hollow = GetStockObject(HOLLOW_BRUSH);
    let old_brush = SelectObject(dc, hollow);
    let pen = CreatePen(PS_SOLID, 2, rgb(0xBED0E9));
    let old = SelectObject(dc, pen.into());
    let _ = Ellipse(dc, center_x - 6, center_y - 6, center_x + 6, center_y + 6);
    let _ = MoveToEx(dc, center_x, center_y - 4, None);
    let _ = LineTo(dc, center_x, center_y);
    let _ = LineTo(dc, center_x + 4, center_y + 2);
    SelectObject(dc, old);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(pen.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn bars_glyph(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, baseline: i32) {
    let pen = CreatePen(PS_SOLID, 4, rgb(0xBED0E9));
    let old = SelectObject(dc, pen.into());
    let _ = MoveToEx(dc, x, baseline, None);
    let _ = LineTo(dc, x, baseline - 9);
    let _ = MoveToEx(dc, x + 8, baseline, None);
    let _ = LineTo(dc, x + 8, baseline - 17);
    let _ = MoveToEx(dc, x + 16, baseline, None);
    let _ = LineTo(dc, x + 16, baseline - 25);
    SelectObject(dc, old);
    let _ = DeleteObject(pen.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn add_tray(hwnd: HWND) -> windows::core::Result<()> {
    let icon = native_icon();
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    let tip: Vec<u16> = tray_tip().encode_utf16().collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    Shell_NotifyIconW(NIM_ADD, &data).ok()?;
    // Standard hover text is only guaranteed after negotiating the current shell protocol.
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    Shell_NotifyIconW(NIM_SETVERSION, &data).ok()
}

fn tray_tip() -> String {
    match BATTERY
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.clone())
    {
        Some(ProbeResult::Ok { reading }) => format!(
            "R5 Battery Estimator — {}% — {}",
            reading.percent,
            if reading.charging {
                "Cargando"
            } else {
                "No cargando"
            }
        ),
        _ => "R5 Battery Estimator — sin lectura válida".to_owned(),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn update_tray(hwnd: HWND) {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_TIP | NIF_SHOWTIP,
        ..Default::default()
    };
    let tip: Vec<u16> = tray_tip()
        .encode_utf16()
        .take(data.szTip.len() - 1)
        .collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn remove_tray(hwnd: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}
