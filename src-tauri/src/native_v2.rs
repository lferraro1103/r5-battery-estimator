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
            BlurEffectGuid, BlurParams, CompositingQualityHighQuality, GdipBitmapApplyEffect, GdipCreateBitmapFromFile,
            GdipCreateBitmapFromScan0, GdipCreateEffect, GdipCreateFromHDC,
            GdipCreateHICONFromBitmap, GdipCreatePen1, GdipCreateLineBrushFromRectI,
            GdipCreateSolidFill,
            GdipDeleteBrush, GdipDeleteEffect, GdipDeleteGraphics, GdipDeletePen,
            GdipDisposeImage, GdipDrawArcI, GdipDrawCurve2I, GdipDrawEllipseI, GdipDrawImageRectI, GdipDrawLinesI,
            GdipFillEllipseI,
            GdipFillPolygonI, GdipGetImageGraphicsContext, GdipSetCompositingQuality,
            GdipSetEffectParameters, GdipSetInterpolationMode, GdipSetPenEndCap,
            GdipSetPenStartCap, GdipSetPixelOffsetMode, GdipSetSmoothingMode,
            FillModeWinding, GdiplusStartup, GdiplusStartupInput, InterpolationModeHighQualityBicubic,
            LineCapRound, LinearGradientModeVertical, PixelOffsetModeHighQuality, Point,
            SmoothingModeAntiAlias, UnitPixel, WrapModeTileFlipX,
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
                DestroyIcon, LoadCursorW, LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PostQuitMessage,
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
const HEIGHT: i32 = 920;
const CHROME_HEIGHT: i32 = 48;
const REFRESH: Rect = Rect::new(852, 842, 228, 60);
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
            4 => {
                if let Some(state) = BATTERY.get() {
                    let result = probe_once(&R5HidTransport::new());
                    *state.lock().expect("battery state poisoned") = result.clone();
                    record_sample(&result);
                }
                update_tray(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
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
                70,
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
                54,
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
                70,
                143,
                700,
                42,
                18,
                0xBED0E9,
                false,
            );
            draw_right(dc, "R5 ULTRA", 870, 84, 130, 45, 18, 0xFF5353, true);
            draw_mascot(dc, 1010, 62, 84);
            // The three top instruments form one group centred on the history card.
            // 26 px between ring → duration and duration → autonomy.
            card(dc, 386, 235, 350, 230);
            card(dc, 762, 235, 340, 230);
            chart(dc, &samples);
            button(dc);
            ring(dc, 55, 201, 305, percent, status);
            clock_glyph(dc, 421, 280);
            draw(
                dc,
                "DURACIÓN DE CARGA COMPLETA",
                436,
                265,
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
                412,
                315,
                290,
                62,
                42,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se aprende con tu descarga real.",
                412,
                405,
                290,
                30,
                11,
                0xBED0E9,
                false,
            );
            bars_glyph(dc, 799, 291);
            draw(
                dc,
                "AUTONOMÍA RESTANTE",
                823,
                265,
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
                792,
                315,
                270,
                62,
                42,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se recalcula cada 30 segundos.",
                792,
                405,
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
                855,
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
    let _ = AppendMenuW(menu, MF_STRING, 4, w!("Actualizar ahora"));
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
    card(dc, 38, 545, 1064, 280);
    // Header and axis labels share the card's left content margin.
    bars_glyph(dc, 70, 602);
    draw(
        dc,
        "HISTORIAL: HORA / PORCENTAJE",
        98,
        575,
        420,
        30,
        15,
        0xBED0E9,
        true,
    );
    draw_right(dc, "100%", 70, 618, 60, 25, 11, 0xBED0E9, false);
    draw_right(dc, "0%", 90, 752, 40, 25, 11, 0xBED0E9, false);
    draw(dc, "24 h", 133, 780, 80, 25, 11, 0xBED0E9, false);
    draw_right(dc, "ahora", 993, 780, 63, 25, 11, 0xBED0E9, false);
    let grid = CreatePen(PS_SOLID, 1, rgb(0x334956));
    let old = SelectObject(dc, grid.into());
    for row in 0..4 {
        let y = 630 + row * 34;
        let _ = MoveToEx(dc, 133, y, None);
        let _ = LineTo(dc, 1056, y);
    }
    for col in 1..4 {
        let x = 133 + col * 231;
        let _ = MoveToEx(dc, x, 630, None);
        let _ = LineTo(dc, x, 764);
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
            X: 133 + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0) as i32,
            Y: 764 - sample.percent as i32 * 134 / 100,
        }).collect();
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
            let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            // The visual depth belongs exclusively under the data line: a vertical
            // green fade to transparent, not an outline shadow around the series.
            let mut area = points.clone();
            area.push(Point { X: points.last().expect("points not empty").X, Y: 764 });
            area.push(Point { X: points[0].X, Y: 764 });
            let mut brush = std::ptr::null_mut();
            let gradient_bounds = windows::Win32::Graphics::GdiPlus::Rect { X: 133, Y: 630, Width: 923, Height: 134 };
            if GdipCreateLineBrushFromRectI(&gradient_bounds, 0x4A2FE189, 0x002FE189, LinearGradientModeVertical, WrapModeTileFlipX, &mut brush).0 == 0 {
                let _ = GdipFillPolygonI(graphics, brush.cast(), area.as_ptr(), area.len() as i32, FillModeWinding);
                let _ = GdipDeleteBrush(brush.cast());
            }
            draw_gp_curve(graphics, &points, 0xFF2FE189, 3.0);
            let _ = GdipDeleteGraphics(graphics);
        }
    } else if let Some(sample) = visible.first() {
        let x = 133
            + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0) as i32;
        let y = 764 - sample.percent as i32 * 134 / 100;
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
            670,
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
            // This same full-size master is used by the title bar and mascot.  The
            // title bar is its most demanding case, so explicitly choose the high-
            // quality resampler instead of GDI+'s low-quality default.
            let _ = GdipSetInterpolationMode(graphics, InterpolationModeHighQualityBicubic);
            let _ = GdipSetCompositingQuality(graphics, CompositingQualityHighQuality);
            let _ = GdipSetPixelOffsetMode(graphics, PixelOffsetModeHighQuality);
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

/// Small native tray artwork: a compact shark face whose body colour communicates
/// battery state.  It deliberately has no battery on its head; that asset remains
/// reserved for the title bar and panel mascot.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn tray_icon() -> HICON {
    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
    let tint = BATTERY
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| match &*state {
            ProbeResult::Ok { reading } if reading.percent >= 55 => Some(0xff31d783),
            ProbeResult::Ok { reading } if reading.percent >= 25 => Some(0xffffbd45),
            ProbeResult::Ok { .. } => Some(0xffef5350),
            _ => None,
        })
        .unwrap_or(0xff6b7b89);
    let mut bitmap = std::ptr::null_mut();
    if GdipCreateBitmapFromScan0(64, 64, 0, PIXEL_FORMAT_32BPP_ARGB, None, &mut bitmap).0 != 0 {
        return native_icon();
    }
    let mut graphics = std::ptr::null_mut();
    if GdipGetImageGraphicsContext(bitmap.cast(), &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        let _ = GdipSetPixelOffsetMode(graphics, PixelOffsetModeHighQuality);
        fill_gp_polygon(graphics, &[Point { X: 29, Y: 12 }, Point { X: 39, Y: 2 }, Point { X: 42, Y: 21 }], tint);
        fill_gp_polygon(graphics, &[Point { X: 9, Y: 38 }, Point { X: 1, Y: 47 }, Point { X: 17, Y: 48 }], tint);
        fill_gp_polygon(graphics, &[Point { X: 55, Y: 38 }, Point { X: 63, Y: 47 }, Point { X: 47, Y: 48 }], tint);
        fill_gp_ellipse(graphics, 7, 14, 50, 43, tint);
        fill_gp_ellipse(graphics, 13, 34, 38, 20, 0xfff5fbff);
        fill_gp_ellipse(graphics, 19, 29, 8, 10, 0xff07131c);
        fill_gp_ellipse(graphics, 38, 29, 8, 10, 0xff07131c);
        fill_gp_ellipse(graphics, 21, 30, 2, 3, 0xffffffff);
        fill_gp_ellipse(graphics, 40, 30, 2, 3, 0xffffffff);
        draw_gp_arc(graphics, 23, 38, 18, 0xff07131c, 2.2, 5.0, 170.0, true);
        let _ = GdipDeleteGraphics(graphics);
    }
    let mut icon = HICON::default();
    let status = GdipCreateHICONFromBitmap(bitmap.cast(), &mut icon);
    let _ = GdipDisposeImage(bitmap.cast());
    if status.0 == 0 { icon } else { native_icon() }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn fill_gp_ellipse(
    graphics: *mut windows::Win32::Graphics::GdiPlus::GpGraphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
) {
    let mut brush = std::ptr::null_mut();
    if GdipCreateSolidFill(color, &mut brush).0 == 0 {
        let _ = GdipFillEllipseI(graphics, brush.cast(), x, y, width, height);
        let _ = GdipDeleteBrush(brush.cast());
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn fill_gp_polygon(
    graphics: *mut windows::Win32::Graphics::GdiPlus::GpGraphics,
    points: &[Point],
    color: u32,
) {
    let mut brush = std::ptr::null_mut();
    if GdipCreateSolidFill(color, &mut brush).0 == 0 {
        let _ = GdipFillPolygonI(graphics, brush.cast(), points.as_ptr(), points.len() as i32, FillModeWinding);
        let _ = GdipDeleteBrush(brush.cast());
    }
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
    // Two concentric rings: a decorative outer instrument ring and a separate,
    // smaller inner progress ring. Their gap is intentional and always visible.
    let outer_left = x + 14;
    let outer_top = y + 14;
    let outer_diameter = size - 28;
    let left = x + 43;
    let top = y + 43;
    let diameter = size - 86;
    let center_x = x + size / 2;
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        draw_gp_arc(graphics, outer_left, outer_top, outer_diameter, 0xff27333e, 3.0, -90.0, 359.95, false);
        draw_gp_arc(graphics, left, top, diameter, 0xff24323d, 18.0, -90.0, 359.95, true);
        if let Some(value) = percent.filter(|value| *value > 0) {
            let sweep = value as f32 * 3.6;
            // A raster mask is actually blurred with the GDI+ Blur effect before composition.
            draw_blurred_arc(dc, x, y, size, left - x, top - y, diameter, sweep);
            // The shaded base plus the narrow highlight form one illuminated,
            // three-dimensional progress surface.  They are inside the real blurred
            // mask below, so this is not a detached shadow or a third ring.
            draw_gp_arc(
                graphics,
                left,
                top,
                diameter,
                0xff1abd74,
                22.0,
                -90.0,
                sweep.min(359.95),
                value < 100,
            );
            draw_gp_arc(
                graphics,
                left,
                top,
                diameter,
                0xff45f4a1,
                15.0,
                -90.0,
                sweep.min(359.95),
                value < 100,
            );
            if value == 100 {
                draw_gp_arc(
                    graphics, left, top, diameter, 0xff1abd74, 22.0, -90.0, 359.95, false,
                );
                draw_gp_arc(
                    graphics, left, top, diameter, 0xff45f4a1, 15.0, -90.0, 359.95, false,
                );
            }
        }
        let _ = GdipDeleteGraphics(graphics);
    }
    let (value, value_size) = percent
        .map(|value| (format!("{value}%"), 42))
        .unwrap_or_else(|| ("—%".to_owned(), 38));
    draw_center(dc, &value, x + 50, y + 102, 205, 62, value_size, 0xF7F9FC, true);
    battery_glyph(dc, center_x, y + 173, percent.is_some());
    draw_center(dc, status, x + 48, y + 202, 210, 28, 15, 0xBED0E9, false);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_blurred_arc(
    target: windows::Win32::Graphics::Gdi::HDC,
    x: i32,
    y: i32,
    size: i32,
    left: i32,
    top: i32,
    diameter: i32,
    sweep: f32,
) {
    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
    let mut bitmap = std::ptr::null_mut();
    if GdipCreateBitmapFromScan0(size, size, 0, PIXEL_FORMAT_32BPP_ARGB, None, &mut bitmap).0 != 0 { return; }
    let mut mask_graphics = std::ptr::null_mut();
    if GdipGetImageGraphicsContext(bitmap.cast(), &mut mask_graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(mask_graphics, SmoothingModeAntiAlias);
        // Render a broad, opaque source then blur it through GDI+.  The result is
        // a true soft-light halo; it stays behind the two physical rings.
        draw_gp_arc(mask_graphics, left, top, diameter, 0xF43BEE99, 30.0, -90.0, sweep.min(359.95), true);
        let _ = GdipDeleteGraphics(mask_graphics);
    }
    let mut effect = std::ptr::null_mut();
    if GdipCreateEffect(BlurEffectGuid, &mut effect).0 == 0 {
        let params = BlurParams { radius: 22.0, expandEdge: true.into() };
        let _ = GdipSetEffectParameters(effect, (&params as *const BlurParams).cast(), size_of::<BlurParams>() as u32);
        let mut roi = windows::Win32::Foundation::RECT { left: 0, top: 0, right: size, bottom: size };
        let _ = GdipBitmapApplyEffect(bitmap, effect, &mut roi, false, std::ptr::null_mut(), std::ptr::null_mut());
        let _ = GdipDeleteEffect(effect);
    }
    let mut target_graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(target, &mut target_graphics).0 == 0 {
        let _ = GdipDrawImageRectI(target_graphics, bitmap.cast(), x, y, size, size);
        let _ = GdipDeleteGraphics(target_graphics);
    }
    let _ = GdipDisposeImage(bitmap.cast());
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
unsafe fn draw_gp_curve(
    graphics: *mut windows::Win32::Graphics::GdiPlus::GpGraphics,
    points: &[Point],
    argb: u32,
    width: f32,
) {
    let mut pen = std::ptr::null_mut();
    if GdipCreatePen1(argb, width, UnitPixel, &mut pen).0 == 0 {
        let _ = GdipSetPenStartCap(pen, LineCapRound);
        let _ = GdipSetPenEndCap(pen, LineCapRound);
        // Tension below 0.5 keeps real samples recognisable while avoiding sharp joins.
        let _ = GdipDrawCurve2I(graphics, pen, points.as_ptr(), points.len() as i32, 0.30);
        let _ = GdipDeletePen(pen);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_gp_polyline(
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
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        let _ = GdipSetPixelOffsetMode(graphics, PixelOffsetModeHighQuality);
        let mut pen = std::ptr::null_mut();
        if GdipCreatePen1(0xFFBED0E9, 1.75, UnitPixel, &mut pen).0 == 0 {
            let _ = GdipDrawEllipseI(graphics, pen, center_x - 7, center_y - 7, 14, 14);
            let _ = GdipDeletePen(pen);
        }
        draw_gp_polyline(graphics, &[Point { X: center_x, Y: center_y - 4 }, Point { X: center_x, Y: center_y }, Point { X: center_x + 4, Y: center_y + 2 }], 0xFFBED0E9, 2.0);
        let _ = GdipDeleteGraphics(graphics);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn bars_glyph(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, baseline: i32) {
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        for (offset, height) in [(0, 9), (8, 17), (16, 25)] {
            draw_gp_polyline(graphics, &[Point { X: x + offset, Y: baseline }, Point { X: x + offset, Y: baseline - height }], 0xFFBED0E9, 3.5);
        }
        let _ = GdipDeleteGraphics(graphics);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn add_tray(hwnd: HWND) -> windows::core::Result<()> {
    let icon = tray_icon();
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
    let _ = DestroyIcon(icon);
    // Standard hover text is only guaranteed after negotiating the current shell protocol.
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    Shell_NotifyIconW(NIM_SETVERSION, &data).ok()
}

fn tray_tip() -> String {
    let learned = HISTORY
        .get()
        .and_then(|history| history.lock().ok())
        .map(|samples| learned_hours(&samples));
    match BATTERY
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.clone())
    {
        Some(ProbeResult::Ok { reading }) => format!(
            "R5 Battery Estimator — {}% — {} — Autonomía: {:.0} h",
            reading.percent,
            if reading.charging {
                "Cargando"
            } else {
                "No cargando"
            },
            remaining_hours(learned.flatten(), reading.percent),
        ),
        _ => "R5 Battery Estimator — sin lectura válida".to_owned(),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn update_tray(hwnd: HWND) {
    let icon = tray_icon();
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        hIcon: icon,
        ..Default::default()
    };
    let tip: Vec<u16> = tray_tip()
        .encode_utf16()
        .take(data.szTip.len() - 1)
        .collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    let _ = DestroyIcon(icon);
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
