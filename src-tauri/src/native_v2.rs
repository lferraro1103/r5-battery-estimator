//! V2 native Windows shell: no .NET, Chromium, WebView, or local HTTP server.
// Release builds are ordinary graphical Windows applications, so launching the
// portable executable never opens a companion console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use chrono::DateTime;
use r5_battery_estimator::{battery::transport::R5HidTransport, probe_once, ProbeResult};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    mem::{size_of, MaybeUninit},
    path::{Path, PathBuf},
    sync::{mpsc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, COLORREF, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteObject,
            DrawTextW, Ellipse, EndPaint, FillRect, GetStockObject, InvalidateRect, LineTo,
            MoveToEx, RoundRect, SelectObject, SetBkMode, SetTextColor, SetWindowRgn,
            CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER,
            DT_LEFT, DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_NORMAL, HOLLOW_BRUSH,
            OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
        },
        Graphics::GdiPlus::{
            BlurEffectGuid, BlurParams, CompositingQualityHighQuality, FillModeWinding,
            GdipBitmapApplyEffect, GdipCreateBitmapFromFile, GdipCreateBitmapFromScan0,
            GdipCreateEffect, GdipCreateFromHDC, GdipCreateHICONFromBitmap,
            GdipCreateLineBrushFromRectI, GdipCreatePen1, GdipCreateSolidFill, GdipDeleteBrush,
            GdipDeleteEffect, GdipDeleteGraphics, GdipDeletePen, GdipDisposeImage, GdipDrawArcI,
            GdipDrawCurve2I, GdipDrawEllipseI, GdipDrawImageRectI, GdipDrawLinesI,
            GdipFillEllipseI, GdipFillPolygonI, GdipGetImageGraphicsContext,
            GdipSetCompositingQuality, GdipSetEffectParameters, GdipSetInterpolationMode,
            GdipSetPenEndCap, GdipSetPenStartCap, GdipSetPixelOffsetMode, GdipSetSmoothingMode,
            GdiplusStartup, GdiplusStartupInput, InterpolationModeHighQualityBicubic, LineCapRound,
            LinearGradientModeVertical, PixelOffsetModeHighQuality, Point, SmoothingModeAntiAlias,
            UnitPixel, WrapModeTileFlipX,
        },
        System::Threading::CreateMutexW,
        Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
        System::{
            LibraryLoader::GetModuleHandleW,
            Registry::{
                RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
                RRF_RT_REG_SZ,
            },
        },
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            Shell::{
                Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
                DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
                LoadCursorW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
                SendMessageW, SetForegroundWindow, ShowWindow, TrackPopupMenu,
                TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HICON, HTCAPTION,
                IDC_ARROW, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, SW_SHOW,
                WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP,
                WNDCLASSW, WS_POPUP, WS_VISIBLE,
            },
        },
    },
};

const CLASS: windows::core::PCWSTR = w!("R5BatteryEstimatorNativeV2");
const TRAY_MESSAGE: u32 = WM_APP + 17;
const BATTERY_UPDATED: u32 = WM_APP + 18;
const POLL_INTERVAL: Duration = Duration::from_secs(30);
const WIDTH: i32 = 1140;
const HEIGHT: i32 = 916;
const CHROME_HEIGHT: i32 = 44;
const REFRESH: Rect = Rect::new(874, 838, 228, 60);
const MINIMIZE: Rect = Rect::new(1028, 0, 56, 44);
const CLOSE: Rect = Rect::new(1084, 0, 56, 44);
const TRAY_TOGGLE_AUTOSTART: usize = 5;
static BATTERY: OnceLock<Mutex<ProbeResult>> = OnceLock::new();
static HISTORY: OnceLock<Mutex<Vec<Sample>>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static REFRESH_REQUEST: OnceLock<mpsc::SyncSender<()>> = OnceLock::new();
static LAST_READING: OnceLock<Mutex<Option<u64>>> = OnceLock::new();

fn request_refresh() {
    if let Some(queue) = REFRESH_REQUEST.get() {
        // A bounded queue coalesces repeated clicks while a query is in progress.
        let _ = queue.try_send(());
    }
}

fn advance_deadline(mut deadline: Instant, now: Instant) -> Instant {
    while deadline <= now {
        deadline += POLL_INTERVAL;
    }
    deadline
}

fn start_worker(hwnd: HWND) {
    let (sender, requests) = mpsc::sync_channel(1);
    let _ = REFRESH_REQUEST.set(sender);
    let window = hwnd.0 as usize;
    thread::spawn(move || {
        let transport = R5HidTransport::new();
        let mut deadline = Instant::now();
        loop {
            let wait = deadline.saturating_duration_since(Instant::now());
            match requests.recv_timeout(wait) {
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let result = probe_once(&transport);
            // Clicks arriving during this transaction are satisfied by this reading.
            while requests.try_recv().is_ok() {}
            if !matches!(result, ProbeResult::Error { code: "busy", .. }) {
                record_sample(&result);
                if matches!(result, ProbeResult::Ok { .. }) {
                    *LAST_READING.get().unwrap().lock().unwrap() = Some(now_seconds());
                }
                *BATTERY.get().unwrap().lock().unwrap() = result;
                unsafe {
                    if PostMessageW(Some(HWND(window as *mut _)), BATTERY_UPDATED, WPARAM(0), LPARAM(0)).is_err() {
                        break;
                    }
                }
            }
            deadline = advance_deadline(deadline, Instant::now());
        }
    });
}

struct BrandImages {
    shark: *mut windows::Win32::Graphics::GdiPlus::GpImage,
}
thread_local! {
    static BRAND_IMAGES: RefCell<Option<BrandImages>> = const { RefCell::new(None) };
}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        BATTERY.get_or_init(|| Mutex::new(ProbeResult::Error {
            code: "pending", message: "Esperando lectura".to_owned(),
        }));
        LAST_READING.get_or_init(|| Mutex::new(None));
        HISTORY.get_or_init(|| Mutex::new(load_history()));
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
        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        let _ = TASKBAR_CREATED.set(taskbar_created);
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
        start_worker(hwnd);
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
        taskbar_created if TASKBAR_CREATED.get().copied() == Some(taskbar_created) => {
            // Explorer discards its notification-area registrations when the taskbar
            // is recreated. Re-add the current icon and renegotiate protocol v4.
            let _ = add_tray(hwnd);
            LRESULT(0)
        }
        // NOTIFYICON_VERSION_4 stores the mouse event in the low word and the
        // icon id in the high word.  Right clicks commonly arrive as CONTEXTMENU.
        TRAY_MESSAGE if tray_event(lparam) == WM_LBUTTONUP => {
            show_tray_menu(hwnd);
            LRESULT(0)
        }
        TRAY_MESSAGE if matches!(tray_event(lparam), WM_RBUTTONUP | WM_CONTEXTMENU) => {
            show_tray_menu(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => match (wparam.0 & 0xffff) as usize {
            1 => {
                let _ = ShowWindow(hwnd, SW_SHOW);
                LRESULT(0)
            }
            2 => {
                LRESULT(0)
            }
            3 => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            4 => {
                request_refresh();
                LRESULT(0)
            }
            TRAY_TOGGLE_AUTOSTART => {
                let _ = set_autostart(!autostart_enabled());
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
                request_refresh();
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
        BATTERY_UPDATED => {
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
                38,
                64,
                700,
                70,
                42,
                0xF7F9FC,
                true,
            );
            draw_brand_icon(dc, 20, 8, 28);
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
                38,
                125,
                700,
                42,
                18,
                0xBED0E9,
                false,
            );
            draw_right(dc, "R5 ULTRA", 874, 84, 130, 45, 18, 0xFF5353, true);
            draw_mascot(dc, 1010, 58, 84);
            // The three top instruments use 34 px between each visible neighbour.
            card(dc, 349, 220, 350, 230);
            card(dc, 733, 220, 340, 230);
            chart(dc, &samples);
            button(dc);
            // The actual outer ring begins at x + 14, matching the graph border x = 38.
            ring(dc, 24, 182, 305, percent, status);
            clock_glyph(dc, 384, 265);
            draw(
                dc,
                "DURACIÓN DE CARGA COMPLETA",
                399,
                250,
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
                375,
                300,
                290,
                62,
                42,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se aprende con tu descarga real.",
                375,
                390,
                290,
                30,
                11,
                0xBED0E9,
                false,
            );
            bars_glyph(dc, 770, 276);
            draw(
                dc,
                "AUTONOMÍA RESTANTE",
                794,
                250,
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
                763,
                300,
                270,
                62,
                42,
                0xF5F5F5,
                true,
            );
            draw(
                dc,
                "Se recalcula cada 30 segundos.",
                763,
                390,
                280,
                30,
                11,
                0xBED0E9,
                false,
            );
            draw(
                dc,
                &last_reading_label(),
                55,
                851,
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

fn tray_event(lparam: LPARAM) -> u32 {
    (lparam.0 as u32) & 0xffff
}

fn last_reading_label() -> String {
    LAST_READING.get().and_then(|state| state.lock().ok()).and_then(|at| *at)
        .and_then(|at| DateTime::from_timestamp(at as i64, 0))
        .map(|at| format!("Última lectura válida: {} · Sondeo cada 30 s", at.with_timezone(&chrono::Local).format("%H:%M:%S")))
        .unwrap_or_else(|| "Esperando lectura válida · Sondeo cada 30 s".to_owned())
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
    let autostart_label = if autostart_enabled() {
        "Iniciar con Windows: activado"
    } else {
        "Iniciar con Windows"
    };
    let autostart_label: Vec<u16> = autostart_label.encode_utf16().chain(Some(0)).collect();
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        TRAY_TOGGLE_AUTOSTART,
        PCWSTR(autostart_label.as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, w!(""));
    let _ = AppendMenuW(menu, MF_STRING, 3, w!("Cerrar programa"));
    let mut point = windows::Win32::Foundation::POINT::default();
    if GetCursorPos(&mut point).is_ok() {
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, Default::default(), point.x, point.y, None, hwnd, None);
    }
    let _ = DestroyMenu(menu);
}

/// The tray is always present, so its command is the only place where the
/// per-user startup preference lives.  This uses the standard HKCU Run entry:
/// no administrator privileges, service, console process, or scheduled task.
fn autostart_enabled() -> bool {
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            w!("R5BatteryEstimator"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
        .is_ok()
    }
}

fn set_autostart(enabled: bool) -> bool {
    unsafe {
        if !enabled {
            return RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
                w!("R5BatteryEstimator"),
            )
            .is_ok()
                || !autostart_enabled();
        }

        let Ok(executable) = std::env::current_exe() else {
            return false;
        };
        let command = format!("\\\"{}\\\"", executable.display());
        let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            w!("R5BatteryEstimator"),
            REG_SZ.0,
            Some(wide.as_ptr().cast()),
            (wide.len() * size_of::<u16>()) as u32,
        )
        .is_ok()
    }
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
    card(dc, 38, 530, 1064, 280);
    // Header and axis labels share the card's left content margin.
    bars_glyph(dc, 70, 587);
    draw(
        dc,
        "HISTORIAL: HORA / PORCENTAJE",
        98,
        560,
        420,
        30,
        15,
        0xBED0E9,
        true,
    );
    draw_right(dc, "100%", 70, 603, 60, 25, 11, 0xBED0E9, false);
    draw_right(dc, "0%", 90, 737, 40, 25, 11, 0xBED0E9, false);
    draw(dc, "24 h", 133, 765, 80, 25, 11, 0xBED0E9, false);
    draw_right(dc, "ahora", 993, 765, 63, 25, 11, 0xBED0E9, false);
    let grid = CreatePen(PS_SOLID, 1, rgb(0x334956));
    let old = SelectObject(dc, grid.into());
    for row in 0..4 {
        let y = 615 + row * 34;
        let _ = MoveToEx(dc, 133, y, None);
        let _ = LineTo(dc, 1056, y);
    }
    for col in 1..4 {
        let x = 133 + col * 231;
        let _ = MoveToEx(dc, x, 615, None);
        let _ = LineTo(dc, x, 749);
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
        let points: Vec<Point> = visible
            .iter()
            .map(|sample| Point {
                X: 133
                    + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0)
                        as i32,
                Y: 749 - sample.percent as i32 * 134 / 100,
            })
            .collect();
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
            let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            // The visual depth belongs exclusively under the data line: a vertical
            // green fade to transparent, not an outline shadow around the series.
            let mut area = points.clone();
            area.push(Point {
                X: points.last().expect("points not empty").X,
                Y: 749,
            });
            area.push(Point {
                X: points[0].X,
                Y: 749,
            });
            let mut brush = std::ptr::null_mut();
            let gradient_bounds = windows::Win32::Graphics::GdiPlus::Rect {
                X: 133,
                Y: 615,
                Width: 923,
                Height: 134,
            };
            if GdipCreateLineBrushFromRectI(
                &gradient_bounds,
                0x4A2FE189,
                0x002FE189,
                LinearGradientModeVertical,
                WrapModeTileFlipX,
                &mut brush,
            )
            .0 == 0
            {
                let _ = GdipFillPolygonI(
                    graphics,
                    brush.cast(),
                    area.as_ptr(),
                    area.len() as i32,
                    FillModeWinding,
                );
                let _ = GdipDeleteBrush(brush.cast());
            }
            draw_gp_curve(graphics, &points, 0xFF2FE189, 3.0);
            let _ = GdipDeleteGraphics(graphics);
        }
    } else if let Some(sample) = visible.first() {
        let x = 133
            + (((sample.at.saturating_sub(start)) as f64 / (24.0 * 60.0 * 60.0)) * 923.0) as i32;
        let y = 749 - sample.percent as i32 * 134 / 100;
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
            655,
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
        .join("assets")
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
        let image = images
            .as_ref()
            .map(|assets| assets.shark)
            .unwrap_or(std::ptr::null_mut());
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
            if !images.shark.is_null() {
                let _ = GdipDisposeImage(images.shark);
            }
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
        fill_gp_polygon(
            graphics,
            &[
                Point { X: 29, Y: 12 },
                Point { X: 39, Y: 2 },
                Point { X: 42, Y: 21 },
            ],
            tint,
        );
        fill_gp_polygon(
            graphics,
            &[
                Point { X: 9, Y: 38 },
                Point { X: 1, Y: 47 },
                Point { X: 17, Y: 48 },
            ],
            tint,
        );
        fill_gp_polygon(
            graphics,
            &[
                Point { X: 55, Y: 38 },
                Point { X: 63, Y: 47 },
                Point { X: 47, Y: 48 },
            ],
            tint,
        );
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
    if status.0 == 0 {
        icon
    } else {
        native_icon()
    }
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
        let _ = GdipFillPolygonI(
            graphics,
            brush.cast(),
            points.as_ptr(),
            points.len() as i32,
            FillModeWinding,
        );
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

fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn load_history() -> Vec<Sample> {
    if let Some(history) = fs::read_to_string(history_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
    {
        return normalize_history(history, now_seconds());
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
    normalize_history(migrated, now_seconds())
}

fn normalize_history(mut samples: Vec<Sample>, now: u64) -> Vec<Sample> {
    let cutoff = now.saturating_sub(14 * 24 * 60 * 60);
    samples.retain(|sample| sample.percent <= 100 && sample.at >= cutoff && sample.at <= now);
    samples.sort_by_key(|sample| sample.at);
    // Conflicting duplicate timestamps cannot represent an observed sequence.
    samples.dedup_by_key(|sample| sample.at);
    samples
}

fn replace_history(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(target.as_ptr()), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
            .map_err(io::Error::other)
    }
}

fn save_history_with(
    path: &Path, samples: &[Sample], replace: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or_else(|| io::Error::other("history needs a directory"))?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
    let result = (|| {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        let raw = serde_json::to_vec(samples).map_err(io::Error::other)?;
        file.write_all(&raw)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn record_sample(result: &ProbeResult) {
    let ProbeResult::Ok { reading } = result else {
        return;
    };
    let Some(history) = HISTORY.get() else { return };
    let mut items = history.lock().expect("history poisoned");
    let now = now_seconds();
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
    *items = normalize_history(std::mem::take(&mut *items), now);
    let snapshot = items.clone();
    drop(items); // Never hold a lock needed by painting during disk I/O.
    let _ = save_history_with(&history_path(), &snapshot, replace_history);
}

fn learned_hours(samples: &[Sample]) -> Option<f64> {
    let mut start: Option<&Sample> = None;
    let mut previous: Option<&Sample> = None;
    let mut elapsed = 0_u64;
    let mut dropped = 0_u64;
    for sample in samples {
        let boundary = sample.charging || sample.percent > 100 || previous.is_some_and(|before| {
            sample.at <= before.at || sample.at - before.at > 600 || sample.percent > before.percent
        });
        if boundary {
            start = None;
        }
        if sample.charging || sample.percent > 100 {
            previous = None;
            continue;
        }
        if start.is_none() && sample.percent >= 95 {
            start = Some(sample);
        }
        if let Some(full) = start.filter(|_| sample.percent <= 5) {
            let cycle_elapsed = sample.at - full.at;
            let cycle_drop = (full.percent - sample.percent) as u64;
            if cycle_elapsed >= 1800 && cycle_drop >= 90 {
                elapsed += cycle_elapsed;
                dropped += cycle_drop;
            }
            // Count a completed cycle once, never combine its low tail with another cycle.
            start = None;
        }
        previous = Some(sample);
    }
    if dropped == 0 {
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
    use super::{advance_deadline, learned_hours, Sample, POLL_INTERVAL};

    #[test]
    fn polling_deadline_does_not_add_query_duration_or_manual_refresh() {
        let start = std::time::Instant::now();
        let next = advance_deadline(start, start + std::time::Duration::from_secs(2));
        assert_eq!(next, start + POLL_INTERVAL);
        assert_eq!(advance_deadline(next, start + std::time::Duration::from_secs(10)), next);
        assert_eq!(advance_deadline(next, start + std::time::Duration::from_secs(75)), start + POLL_INTERVAL * 3);
    }

    fn cycle(at: u64, spacing: u64) -> Vec<Sample> {
        (5..=100).rev().enumerate().map(|(index, percent)| Sample {
            at: at + index as u64 * spacing, percent, charging: false,
        }).collect()
    }

    #[test]
    fn loaded_history_repairs_order_duplicates_ranges_and_dates() {
        let now = 2_000_000;
        let samples = vec![
            Sample { at: now, percent: 50, charging: false },
            Sample { at: now - 30, percent: 51, charging: false },
            Sample { at: now, percent: 50, charging: false },
            Sample { at: now + 1, percent: 40, charging: false },
            Sample { at: now - 60, percent: 101, charging: false },
            Sample { at: 1, percent: 100, charging: false },
        ];
        let repaired = super::normalize_history(samples, now);
        assert_eq!(repaired.len(), 2);
        assert_eq!(repaired[0].at, now - 30);
        assert_eq!(repaired[1].at, now);
    }

    #[test]
    fn history_atomic_roundtrip_and_failed_replacement_preserve_previous_file() {
        let directory = std::env::temp_dir().join(format!("r5-history-test-{}-{}", std::process::id(), super::now_seconds()));
        let path = directory.join("history.json");
        let original = cycle(100, 300);
        super::save_history_with(&path, &original, super::replace_history).unwrap();
        let raw = std::fs::read(&path).unwrap();
        let restored: Vec<Sample> = serde_json::from_slice(&raw).unwrap();
        assert_eq!(restored, original);
        let next = cycle(30_000, 600);
        let failed = super::save_history_with(&path, &next, |temporary, _| {
            let staged: Vec<Sample> = serde_json::from_slice(&std::fs::read(temporary)?).unwrap();
            assert_eq!(staged, next);
            Err(std::io::Error::other("injected replacement failure"))
        });
        assert!(failed.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), raw);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        super::save_history_with(&path, &next, super::replace_history).unwrap();
        assert_eq!(serde_json::from_slice::<Vec<Sample>>(&std::fs::read(&path).unwrap()).unwrap(), next);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn recharge_increase_and_gap_each_invalidate_the_entire_partial_cycle() {
        for boundary in [0, 1, 2] {
            let mut samples = cycle(0, 300);
            match boundary {
                0 => samples[50].charging = true,
                1 => samples[50].percent = 80,
                _ => for item in &mut samples[50..] { item.at += 900; },
            }
            assert_eq!(learned_hours(&samples), None, "boundary {boundary}");
        }
    }

    #[test]
    fn completed_cycles_are_weighted_without_incomplete_recharge_segments() {
        let mut samples = cycle(0, 300);
        samples.push(Sample { at: 28_800, percent: 60, charging: true });
        let mut incomplete = cycle(29_100, 300);
        incomplete.truncate(30);
        samples.extend(incomplete);
        samples.push(Sample { at: 38_100, percent: 100, charging: true });
        samples.extend(cycle(38_400, 600));
        assert!((learned_hours(&samples).unwrap() - 12.5).abs() < 0.001);
    }

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
        draw_gp_arc(
            graphics,
            outer_left,
            outer_top,
            outer_diameter,
            0xff27333e,
            3.0,
            -90.0,
            359.95,
            false,
        );
        draw_gp_arc(
            graphics, left, top, diameter, 0xff24323d, 18.0, -90.0, 359.95, true,
        );
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
    draw_center(
        dc,
        &value,
        x + 50,
        y + 102,
        205,
        62,
        value_size,
        0xF7F9FC,
        true,
    );
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
    if GdipCreateBitmapFromScan0(size, size, 0, PIXEL_FORMAT_32BPP_ARGB, None, &mut bitmap).0 != 0 {
        return;
    }
    let mut mask_graphics = std::ptr::null_mut();
    if GdipGetImageGraphicsContext(bitmap.cast(), &mut mask_graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(mask_graphics, SmoothingModeAntiAlias);
        // Render a broad, opaque source then blur it through GDI+.  The result is
        // a true soft-light halo; it stays behind the two physical rings.
        draw_gp_arc(
            mask_graphics,
            left,
            top,
            diameter,
            0xF43BEE99,
            30.0,
            -90.0,
            sweep.min(359.95),
            true,
        );
        let _ = GdipDeleteGraphics(mask_graphics);
    }
    let mut effect = std::ptr::null_mut();
    if GdipCreateEffect(BlurEffectGuid, &mut effect).0 == 0 {
        let params = BlurParams {
            radius: 22.0,
            expandEdge: true.into(),
        };
        let _ = GdipSetEffectParameters(
            effect,
            (&params as *const BlurParams).cast(),
            size_of::<BlurParams>() as u32,
        );
        let mut roi = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        let _ = GdipBitmapApplyEffect(
            bitmap,
            effect,
            &mut roi,
            false,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
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
        draw_gp_polyline(
            graphics,
            &[
                Point {
                    X: center_x,
                    Y: center_y - 4,
                },
                Point {
                    X: center_x,
                    Y: center_y,
                },
                Point {
                    X: center_x + 4,
                    Y: center_y + 2,
                },
            ],
            0xFFBED0E9,
            2.0,
        );
        let _ = GdipDeleteGraphics(graphics);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn bars_glyph(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, baseline: i32) {
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
        let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        for (offset, height) in [(0, 9), (8, 17), (16, 25)] {
            draw_gp_polyline(
                graphics,
                &[
                    Point {
                        X: x + offset,
                        Y: baseline,
                    },
                    Point {
                        X: x + offset,
                        Y: baseline - height,
                    },
                ],
                0xFFBED0E9,
                3.5,
            );
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
    if let Err(error) = Shell_NotifyIconW(NIM_ADD, &data).ok() {
        let _ = DestroyIcon(icon);
        return Err(error);
    }
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
    if Shell_NotifyIconW(NIM_MODIFY, &data).as_bool() {
        let _ = DestroyIcon(icon);
    } else {
        // The shell may have dropped its registration (for example, during an
        // Explorer restart). NIM_MODIFY cannot recreate a missing notification icon.
        let _ = DestroyIcon(icon);
        let _ = add_tray(hwnd);
    }
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
