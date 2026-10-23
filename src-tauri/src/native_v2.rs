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
        Foundation::{GetLastError, COLORREF, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteObject,
            CreateCompatibleDC, CreateCompatibleBitmap, DeleteDC, GetMonitorInfoW, MonitorFromWindow,
            SetStretchBltMode, StretchBlt, HALFTONE, MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY,
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
            GdipDrawEllipseI, GdipDrawImageRectI, GdipDrawLinesI,
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
            HiDpi::{GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            Input::KeyboardAndMouse::ReleaseCapture,
            Shell::{
                Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_WARNING, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
                DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, GetClientRect, GetWindowRect,
                LoadCursorW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
                SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, TrackPopupMenu,
                TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HICON, HTCAPTION,
                IDC_ARROW, IDI_APPLICATION, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, SW_SHOW,
                WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP,
                WM_DPICHANGED, WM_DISPLAYCHANGE, WM_EXITSIZEMOVE, WM_SIZE, SWP_NOZORDER, SWP_NOACTIVATE,
                WNDCLASSW, WS_POPUP,
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
const HISTORY_DAYS: u64 = 60;
const TEXT: u32 = 0xF7F9FC;
const MUTED: u32 = 0x8EA3B5;
const GREEN: u32 = 0x2FE189;
const AMBER: u32 = 0xFFB833;
const RED: u32 = 0xFF5C5C;
static BATTERY: OnceLock<Mutex<ProbeResult>> = OnceLock::new();
static HISTORY: OnceLock<Mutex<Vec<Sample>>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static REFRESH_REQUEST: OnceLock<mpsc::SyncSender<()>> = OnceLock::new();
static LAST_READING: OnceLock<Mutex<Option<u64>>> = OnceLock::new();
static LAST_VALID: OnceLock<Mutex<Option<Sample>>> = OnceLock::new();

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
                if let ProbeResult::Ok { reading } = &result {
                    let at = now_seconds();
                    *LAST_READING.get().unwrap().lock().unwrap() = Some(at);
                    *LAST_VALID.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(Sample {
                        at, percent: reading.percent, charging: reading.charging,
                    });
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
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // GDI+ is used only by the static anti-aliased ring. It never runs in the HID worker.
        let mut gdiplus_token = 0usize;
        let startup = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let _ = GdiplusStartup(&mut gdiplus_token, &startup, std::ptr::null_mut());
        let mut args = std::env::args().skip(1);
        if args.next().as_deref() == Some("--render-png") {
            let path = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("panel.png"));
            return render_png(&path);
        }
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
            WS_POPUP,
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
        fit_window(hwnd, None);
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
        WM_DPICHANGED => {
            let suggested = *(lparam.0 as *const RECT);
            fit_window(hwnd, Some(suggested));
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_EXITSIZEMOVE => {
            fit_window(hwnd, None);
            LRESULT(0)
        }
        WM_SIZE => {
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            let radius = (24.0 * client.right as f64 / WIDTH as f64).round() as i32;
            let region = CreateRoundRectRgn(0, 0, client.right, client.bottom, radius, radius);
            if SetWindowRgn(hwnd, Some(region), true) == 0 { let _ = DeleteObject(region.into()); }
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            let (x, y) = logical_point((lparam.0 as u16) as i16 as i32, ((lparam.0 >> 16) as u16) as i16 as i32, client.right, client.bottom);
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
            check_low_battery(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let target = BeginPaint(hwnd, &mut paint);
            // Render the original native design as a single layer, then scale all
            // GDI text, GDI+ art and blur together. Hit tests use the inverse mapping.
            let dc = CreateCompatibleDC(Some(target));
            let bitmap = CreateCompatibleBitmap(target, WIDTH, HEIGHT);
            if dc.0.is_null() || bitmap.0.is_null() {
                if !bitmap.0.is_null() { let _ = DeleteObject(bitmap.into()); }
                if !dc.0.is_null() { let _ = DeleteDC(dc); }
                let _ = EndPaint(hwnd, &paint);
                return LRESULT(0);
            }
            let old_bitmap = SelectObject(dc, bitmap.into());
            render_panel(dc);
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            SetStretchBltMode(target, HALFTONE);
            let _ = StretchBlt(target, 0, 0, client.right, client.bottom, Some(dc), 0, 0, WIDTH, HEIGHT, SRCCOPY);
            SelectObject(dc, old_bitmap);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(dc);
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

/// Development aid: renders the dashboard from the local history's latest
/// reading into a PNG, without a window, tray icon or HID query.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn render_png(path: &Path) -> windows::core::Result<()> {
    use r5_battery_estimator::battery::protocol::{ParsedBattery, ReportLayout};
    use windows::Win32::Graphics::{
        Gdi::{GetDC, ReleaseDC, HPALETTE},
        GdiPlus::{GdipCreateBitmapFromHBITMAP, GdipSaveImageToFile},
    };
    const PNG_ENCODER: windows::core::GUID = windows::core::GUID::from_u128(0x557cf406_1a04_11d3_9a73_0000f81ef32e);
    let samples = load_history();
    let latest = samples.last().cloned();
    BATTERY.get_or_init(|| Mutex::new(match &latest {
        Some(sample) => ProbeResult::Ok { reading: ParsedBattery {
            percent: sample.percent, charging: sample.charging, layout: ReportLayout::Normal,
        } },
        None => ProbeResult::Error { code: "pending", message: String::new() },
    }));
    LAST_READING.get_or_init(|| Mutex::new(latest.as_ref().map(|sample| sample.at)));
    HISTORY.get_or_init(|| Mutex::new(samples));
    let screen = GetDC(None);
    let dc = CreateCompatibleDC(Some(screen));
    let bitmap = CreateCompatibleBitmap(screen, WIDTH, HEIGHT);
    let old = SelectObject(dc, bitmap.into());
    render_panel(dc);
    SelectObject(dc, old);
    let mut image = std::ptr::null_mut();
    let wide: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
    let saved = GdipCreateBitmapFromHBITMAP(bitmap, HPALETTE::default(), &mut image).0 == 0
        && GdipSaveImageToFile(image.cast(), PCWSTR(wide.as_ptr()), &PNG_ENCODER, std::ptr::null()).0 == 0;
    if !image.is_null() {
        let _ = GdipDisposeImage(image.cast());
    }
    release_brand_images();
    let _ = DeleteObject(bitmap.into());
    let _ = DeleteDC(dc);
    ReleaseDC(None, screen);
    if saved { Ok(()) } else { Err(windows::Win32::Foundation::E_FAIL.into()) }
}

/// Paints the whole dashboard at its logical WIDTH × HEIGHT. WM_PAINT scales the
/// result to the window; `--render-png` saves it for visual review.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn render_panel(dc: windows::Win32::Graphics::Gdi::HDC) {
    let background = CreateSolidBrush(rgb(0x0A1117));
    FillRect(dc, &RECT { left: 0, top: 0, right: WIDTH, bottom: HEIGHT }, background);
    let _ = DeleteObject(background.into());
    let header = CreateSolidBrush(rgb(0x0D161D));
    FillRect(dc, &RECT { left: 0, top: 0, right: WIDTH, bottom: CHROME_HEIGHT }, header);
    let _ = DeleteObject(header.into());
    SetBkMode(dc, TRANSPARENT);
    let current = BATTERY.get().and_then(|s| s.lock().ok()).map(|s| s.clone());
    let (percent, status, fresh) = panel_battery(current.as_ref(), last_valid_sample());
    let samples = HISTORY
        .get()
        .and_then(|history| history.lock().ok())
        .map(|items| items.clone())
        .unwrap_or_default();
    let learned = estimate(&samples);
    let now = now_seconds();

    // Window chrome.
    draw_brand_icon(dc, 20, 8, 28);
    draw(dc, "R5 Battery Estimator", 54, 0, 320, CHROME_HEIGHT, 14, MUTED, false);
    draw_center(dc, "—", MINIMIZE.x, 0, MINIMIZE.w, MINIMIZE.h, 18, TEXT, false);
    draw_center(dc, "×", CLOSE.x, 0, CLOSE.w, CLOSE.h, 22, TEXT, false);

    // Header: what this is, and whether the receiver is answering right now.
    draw(dc, "Batería del R5 Ultra", 38, 62, 680, 56, 36, TEXT, true);
    draw(dc, "Autonomía estimada a partir de tu consumo real", 40, 114, 680, 30, 16, MUTED, false);
    let (pill, dot) = match current.as_ref() {
        Some(ProbeResult::Ok { .. }) => ("Receptor 2.4 GHz · en línea", GREEN),
        Some(ProbeResult::Error { code: "reading_unavailable", .. }) => ("Mouse en reposo", AMBER),
        Some(ProbeResult::Error { code: "pending", .. }) => ("Conectando con el receptor…", MUTED),
        _ => ("Receptor no detectado", RED),
    };
    status_pill(dc, pill, dot);
    draw_mascot(dc, 1010, 58, 84);

    // The actual outer ring begins at x + 14, matching the chart border x = 38.
    ring(dc, 24, 182, 305, percent, status);

    let dash = || "—".to_owned();
    let (confidence, confidence_color) = learned.map_or(("Baja", MUTED), |estimate| estimate.confidence.word());
    metric_card(
        dc, 349, 350, true, "DURACIÓN DE UNA CARGA",
        &learned.map_or_else(|| "Calibrando".to_owned(), |estimate| format!("{:.0} h", estimate.active_hours)),
        if learned.is_some() { "de uso activo con la batería al 100%" } else { "se necesita al menos 5% de descarga" },
        [
            ("A tu ritmo", learned.and_then(|estimate| estimate.calendar_days)
                .map_or_else(dash, |days| format!("~{}", format_days(days))), TEXT),
            ("Confianza", confidence.to_owned(), confidence_color),
        ],
    );
    let live = percent.filter(|_| fresh);
    metric_card(
        dc, 733, 340, false, "AUTONOMÍA RESTANTE",
        &live.map_or_else(dash, |value| format!("~{:.0} h", remaining_hours(learned, value))),
        match (live, learned) {
            (None, _) => "esperando una lectura del mouse",
            (Some(_), Some(_)) => "de uso activo",
            (Some(_), None) => "estimación inicial, sin calibrar",
        },
        [
            ("A tu ritmo", live.and_then(|value| remaining_days(learned, value))
                .map_or_else(dash, |days| format!("~{}", format_days(days))), TEXT),
            ("Cargar antes del", live.and_then(|value| charge_by(learned, value, now)).unwrap_or_else(dash), TEXT),
        ],
    );

    chart(dc, &samples, now);
    draw(dc, &last_reading_label(), 40, 851, 700, 30, 13, MUTED, false);
    button(dc);
    // Hairline inside the rounded region keeps the shell intentionally rounded instead of full-bleed square.
    let hollow = GetStockObject(HOLLOW_BRUSH);
    let old_brush = SelectObject(dc, hollow);
    let root_border = CreatePen(PS_SOLID, 1, rgb(0x233440));
    let old_pen = SelectObject(dc, root_border.into());
    let _ = RoundRect(dc, 0, 0, WIDTH - 1, HEIGHT - 1, 24, 24);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(root_border.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn status_pill(dc: windows::Win32::Graphics::Gdi::HDC, text: &str, dot: u32) {
    let (x, y, width, height) = (748, 94, 244, 36);
    let fill = CreateSolidBrush(rgb(0x101B23));
    let border = CreatePen(PS_SOLID, 1, rgb(0x233440));
    let old_brush = SelectObject(dc, fill.into());
    let old_pen = SelectObject(dc, border.into());
    let _ = RoundRect(dc, x, y, x + width, y + height, height, height);
    let dot_fill = CreateSolidBrush(rgb(dot));
    let dot_pen = CreatePen(PS_SOLID, 1, rgb(dot));
    SelectObject(dc, dot_fill.into());
    SelectObject(dc, dot_pen.into());
    let _ = Ellipse(dc, x + 16, y + height / 2 - 5, x + 26, y + height / 2 + 5);
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    for object in [fill.into(), border.into(), dot_fill.into(), dot_pen.into()] {
        let _ = DeleteObject(object);
    }
    draw(dc, text, x + 36, y, width - 48, height, 13, 0xD6E2EE, false);
}

/// One headline figure with its unit, then two labelled detail rows.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn metric_card(
    dc: windows::Win32::Graphics::Gdi::HDC,
    x: i32,
    width: i32,
    clock: bool,
    title: &str,
    value: &str,
    unit: &str,
    rows: [(&str, String, u32); 2],
) {
    let y = 220;
    card(dc, x, y, width, 230);
    let label_x = if clock {
        clock_glyph(dc, x + 35, y + 45);
        x + 50
    } else {
        bars_glyph(dc, x + 37, y + 56);
        x + 61
    };
    draw(dc, title, label_x, y + 30, width - 80, 30, 12, MUTED, true);
    draw(dc, value, x + 26, y + 62, width - 52, 58, 42, TEXT, true);
    draw(dc, unit, x + 28, y + 118, width - 56, 24, 13, MUTED, false);
    let divider = CreatePen(PS_SOLID, 1, rgb(0x233440));
    let old = SelectObject(dc, divider.into());
    let _ = MoveToEx(dc, x + 26, y + 154, None);
    let _ = LineTo(dc, x + width - 26, y + 154);
    SelectObject(dc, old);
    let _ = DeleteObject(divider.into());
    for (index, (label, value, color)) in rows.iter().enumerate() {
        let top = y + 160 + index as i32 * 30;
        draw(dc, label, x + 28, top, width / 2, 28, 14, MUTED, false);
        draw_right(dc, value, x + width / 2, top, width / 2 - 28, 28, 14, *color, true);
    }
}

fn tray_event(lparam: LPARAM) -> u32 {
    (lparam.0 as u32) & 0xffff
}

fn fitted_size(dpi: u32, available_width: i32, available_height: i32) -> (i32, i32) {
    let scale = (dpi.max(96) as f64 / 96.0)
        .min(available_width.max(1) as f64 / WIDTH as f64)
        .min(available_height.max(1) as f64 / HEIGHT as f64);
    ((WIDTH as f64 * scale).floor().max(1.0) as i32, (HEIGHT as f64 * scale).floor().max(1.0) as i32)
}

fn logical_point(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    ((x as f64 * WIDTH as f64 / width.max(1) as f64).floor() as i32,
     (y as f64 * HEIGHT as f64 / height.max(1) as f64).floor() as i32)
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn fit_window(hwnd: HWND, suggested: Option<RECT>) {
    let mut monitor = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    // Apply the suggested origin first so MonitorFromWindow resolves the new monitor.
    if let Some(rect) = suggested {
        let _ = SetWindowPos(hwnd, None, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top, SWP_NOZORDER | SWP_NOACTIVATE);
    }
    if !GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut monitor).as_bool() { return; }
    let work = monitor.rcWork;
    let margin = 12;
    let (width, height) = fitted_size(GetDpiForWindow(hwnd), work.right - work.left - margin * 2, work.bottom - work.top - margin * 2);
    let mut current = RECT::default();
    let _ = GetWindowRect(hwnd, &mut current);
    let x = current.left.clamp(work.left + margin, (work.right - margin - width).max(work.left + margin));
    let y = current.top.clamp(work.top + margin, (work.bottom - margin - height).max(work.top + margin));
    let _ = SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
}

fn last_valid_sample() -> Option<Sample> {
    LAST_VALID.get().and_then(|state| state.lock().ok()).and_then(|sample| sample.clone())
}

fn panel_battery(result: Option<&ProbeResult>, last: Option<Sample>) -> (Option<u8>, &'static str, bool) {
    match result {
        Some(ProbeResult::Ok { reading }) => (Some(reading.percent),
            if reading.charging { "Cargando" } else { "En batería" }, true),
        Some(ProbeResult::Error { code: "reading_unavailable", .. }) =>
            (last.map(|sample| sample.percent), "Último valor conocido", false),
        _ => (None, "Sin lectura", false),
    }
}

fn elapsed_label(seconds: u64) -> String {
    match seconds {
        0..90 => format!("{seconds} s"),
        90..5_400 => format!("{} min", (seconds + 30) / 60),
        _ => format!("{} h", (seconds + 1_800) / 3_600),
    }
}

fn pending_tip(last: Option<Sample>, now: u64) -> String {
    match last {
        Some(sample) => format!("R5 Ultra · {}% hace {} · mouse en reposo",
            sample.percent, elapsed_label(now.saturating_sub(sample.at))),
        None => "R5 Ultra · sin lectura · mové el mouse para despertarlo".to_owned(),
    }
}

fn last_reading_label() -> String {
    LAST_READING.get().and_then(|state| state.lock().ok()).and_then(|at| *at)
        .and_then(|at| DateTime::from_timestamp(at as i64, 0))
        .map(|at| format!("Última lectura: {} · actualización automática cada 30 s", at.with_timezone(&chrono::Local).format("%H:%M")))
        .unwrap_or_else(|| "Esperando la primera lectura · actualización automática cada 30 s".to_owned())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = match CreatePopupMenu() {
        Ok(menu) => menu,
        Err(_) => return,
    };
    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Abrir panel"));
    let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 2, w!("Perfiles: pendiente"));
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
        let command = autostart_command(&executable);
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

fn autostart_command(executable: &Path) -> String {
    format!("\"{}\"", executable.display())
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
unsafe fn chart(dc: windows::Win32::Graphics::Gdi::HDC, samples: &[Sample], now: u64) {
    const DAYS: u64 = 7;
    const SPAN: u64 = DAYS * 24 * 60 * 60;
    let (left, right, top, bottom) = (133, 1056, 556, 756);
    card(dc, 38, 480, 1064, 330);
    bars_glyph(dc, 70, 527);
    draw(dc, "Últimos 7 días", 98, 500, 420, 30, 15, TEXT, true);
    draw_right(dc, "Promedio por hora", 700, 500, 356, 30, 12, MUTED, false);
    let grid = CreatePen(PS_SOLID, 1, rgb(0x233440));
    let old = SelectObject(dc, grid.into());
    for row in 0..=4 {
        let y = top + row * (bottom - top) / 4;
        let _ = MoveToEx(dc, left, y, None);
        let _ = LineTo(dc, right, y);
    }
    let start = now.saturating_sub(SPAN);
    let x_of = |at: u64| left + (at.saturating_sub(start) as f64 / SPAN as f64 * (right - left) as f64) as i32;
    let y_of = |percent: f64| bottom - (percent * (bottom - top) as f64 / 100.0).round() as i32;
    // Day separators sit on local midnights, labelled with the day they open.
    for (at, date) in local_midnights(start, now) {
        let x = x_of(at);
        let _ = MoveToEx(dc, x, top, None);
        let _ = LineTo(dc, x, bottom);
        if x + 50 <= right + 20 {
            draw_center(dc, &spanish_date(date), x - 50, bottom + 10, 100, 24, 12, MUTED, false);
        }
    }
    SelectObject(dc, old);
    let _ = DeleteObject(grid.into());
    for (label, percent) in [("100%", 100.0), ("50%", 50.0), ("0%", 0.0)] {
        draw_right(dc, label, 60, y_of(percent) - 12, 62, 24, 12, MUTED, false);
    }

    let points: Vec<Point> = hourly_average(samples, start, now)
        .into_iter()
        .map(|(at, percent)| Point { X: x_of(at), Y: y_of(percent) })
        .collect();
    match points.len() {
        0 => draw_center(dc, "Todavía no hay lecturas en los últimos 7 días", 330, 640, 560, 24, 14, MUTED, false),
        1 => {
            let brush = CreateSolidBrush(rgb(GREEN));
            let old = SelectObject(dc, brush.into());
            let _ = Ellipse(dc, points[0].X - 4, points[0].Y - 4, points[0].X + 5, points[0].Y + 5);
            SelectObject(dc, old);
            let _ = DeleteObject(brush.into());
        }
        _ => {
            let mut graphics = std::ptr::null_mut();
            if GdipCreateFromHDC(dc, &mut graphics).0 == 0 {
                let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
                // The visual depth belongs exclusively under the data line: a vertical
                // green fade to transparent, not an outline shadow around the series.
                let mut area = points.clone();
                area.push(Point { X: points[points.len() - 1].X, Y: bottom });
                area.push(Point { X: points[0].X, Y: bottom });
                let mut brush = std::ptr::null_mut();
                let bounds = windows::Win32::Graphics::GdiPlus::Rect {
                    X: left, Y: top, Width: right - left, Height: bottom - top,
                };
                if GdipCreateLineBrushFromRectI(
                    &bounds, 0x4A2FE189, 0x002FE189, LinearGradientModeVertical, WrapModeTileFlipX, &mut brush,
                ).0 == 0
                {
                    let _ = GdipFillPolygonI(graphics, brush.cast(), area.as_ptr(), area.len() as i32, FillModeWinding);
                    let _ = GdipDeleteBrush(brush.cast());
                }
                draw_gp_polyline(graphics, &points, 0xFF2FE189, 2.5);
                let _ = GdipDeleteGraphics(graphics);
            }
        }
    }
}

/// Local midnights strictly inside (start, end], with the date each one opens.
fn local_midnights(start: u64, end: u64) -> Vec<(u64, chrono::NaiveDate)> {
    use chrono::TimeZone;
    let Some(first) = DateTime::from_timestamp(start as i64, 0) else { return Vec::new() };
    let mut date = first.with_timezone(&chrono::Local).date_naive();
    let mut midnights = Vec::new();
    while let Some(next) = date.succ_opt() {
        date = next;
        let Some(at) = date.and_hms_opt(0, 0, 0)
            .and_then(|midnight| chrono::Local.from_local_datetime(&midnight).earliest())
            .and_then(|midnight| u64::try_from(midnight.timestamp()).ok())
        else { break };
        if at > end {
            break;
        }
        midnights.push((at, date));
    }
    midnights
}

/// Averages readings per hour so firmware jitter and 30 s polling do not draw
/// a noisy line. Returns (bucket middle, mean percent) for hours with data.
fn hourly_average(samples: &[Sample], start: u64, end: u64) -> Vec<(u64, f64)> {
    let mut buckets: Vec<(u64, u64)> = vec![(0, 0); (end.saturating_sub(start) / 3600 + 1) as usize];
    for sample in samples.iter().filter(|sample| sample.at >= start && sample.at <= end) {
        let bucket = &mut buckets[((sample.at - start) / 3600) as usize];
        bucket.0 += sample.percent as u64;
        bucket.1 += 1;
    }
    buckets
        .into_iter()
        .enumerate()
        .filter(|(_, (_, count))| *count > 0)
        .map(|(index, (sum, count))| ((start + index as u64 * 3600 + 1800).min(end), sum as f64 / count as f64))
        .collect()
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn button(dc: windows::Win32::Graphics::Gdi::HDC) {
    let brush = CreateSolidBrush(rgb(0x1F6FD1));
    let pen = CreatePen(PS_SOLID, 1, rgb(0x2F80E6));
    let old_brush = SelectObject(dc, brush.into());
    let old_pen = SelectObject(dc, pen.into());
    let _ = RoundRect(dc, REFRESH.x, REFRESH.y, REFRESH.x + REFRESH.w, REFRESH.y + REFRESH.h, 16, 16);
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    let _ = DeleteObject(brush.into());
    let _ = DeleteObject(pen.into());
    draw_center(dc, "Actualizar ahora", REFRESH.x, REFRESH.y, REFRESH.w, REFRESH.h, 15, 0xFFFFFF, true);
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
    let fallback = || {
        // winres embeds icon id 1; portable runs retain the full shark even if
        // the adjacent PNG is missing. The Windows icon is the last resort.
        GetModuleHandleW(None).ok()
            .and_then(|instance| LoadIconW(Some(instance.into()), PCWSTR(1usize as *const u16)).ok())
            .unwrap_or_else(|| LoadIconW(None, IDI_APPLICATION).expect("Windows application icon must exist"))
    };
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
    // A charge lasts weeks, so keep enough history to span several of them.
    let cutoff = now.saturating_sub(HISTORY_DAYS * 24 * 60 * 60);
    samples.retain(|sample| sample.percent <= 100 && sample.at >= cutoff && sample.at <= now);
    samples.sort_by_key(|sample| sample.at);
    // Stable sorting retains captured order. Remove only identical observations:
    // conflicting states at the same second must survive so the estimator sees
    // their charging boundary.
    samples.dedup();
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

/// An awake plateau at one percentage keeps one reading per active interval
/// instead of one per poll. A merged pair never spans more than ACTIVE_GAP, so
/// the estimator still counts it as active use and still sees sleep/PC-off gaps.
fn append_sample(items: &mut Vec<Sample>, sample: Sample) {
    let same = |other: &Sample| other.percent == sample.percent && other.charging == sample.charging;
    if let [.., before, last] = items.as_mut_slice() {
        if same(before) && same(last) && sample.at.saturating_sub(before.at) <= ACTIVE_GAP {
            last.at = sample.at;
            return;
        }
    }
    items.push(sample);
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
    append_sample(&mut items, sample);
    *items = normalize_history(std::mem::take(&mut *items), now);
    let snapshot = items.clone();
    drop(items); // Never hold a lock needed by painting during disk I/O.
    let _ = save_history_with(&history_path(), &snapshot, replace_history);
}

/// Consecutive readings at most this far apart mean the mouse stayed awake and
/// polled, so the time between them counts as active use.
const ACTIVE_GAP: u64 = 600;
/// Calendar drain spans nights and PC shutdowns, but not an absence so long
/// that it says nothing about the user's normal rhythm.
const CALENDAR_GAP: u64 = 3 * 24 * 60 * 60;
/// The firmware jitters by one or two points; a larger rise is a recharge even
/// when the charging flag was never observed (cable charging, app closed).
const RECHARGE_JUMP: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    fn label(self) -> &'static str {
        match self {
            Confidence::Low => "confianza baja",
            Confidence::Medium => "confianza media",
            Confidence::High => "confianza alta",
        }
    }

    fn word(self) -> (&'static str, u32) {
        match self {
            Confidence::Low => ("Baja", MUTED),
            Confidence::Medium => ("Media", AMBER),
            Confidence::High => ("Alta", GREEN),
        }
    }
}

/// Ring colours (base, highlight, glow) and the matching flat colour, on the
/// same thresholds as the tray icon so both always agree.
fn level_palette(percent: u8) -> (u32, u32, u32, u32) {
    match percent {
        55.. => (0xff1abd74, 0xff45f4a1, 0xF43BEE99, GREEN),
        25.. => (0xffc98a12, 0xffffc24d, 0xF4FFB833, AMBER),
        _ => (0xffc0392b, 0xffff6b6b, 0xF4FF5C5C, RED),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Estimate {
    /// Hours of awake, polled use that a full 100% charge lasts.
    active_hours: f64,
    /// Calendar days a full charge lasts at the user's observed rhythm.
    calendar_days: Option<f64>,
    confidence: Confidence,
}

/// Older drain counts half as much every week, so a new polling rate or a
/// heavier gaming week shows up in days instead of after the 60-day window.
const RECENCY_HALF_LIFE: f64 = 7.0 * 24.0 * 60.0 * 60.0;

/// Learns the drain rate from every discharge stretch instead of requiring one
/// uninterrupted 100%→0% cycle. Net deltas are summed, so ±1% jitter cancels;
/// charging samples and recharge jumps are boundaries that contribute nothing.
/// Rates are recency-weighted; confidence uses the unweighted evidence.
fn estimate(samples: &[Sample]) -> Option<Estimate> {
    let newest = samples.last()?.at;
    let mut evidence = 0_i64;
    let (mut active_drop, mut active_seconds) = (0.0_f64, 0.0_f64);
    let (mut calendar_drop, mut calendar_seconds) = (0.0_f64, 0.0_f64);
    let mut calendar_span = 0_u64;
    for pair in samples.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        let gap = after.at.saturating_sub(before.at);
        if before.charging || after.charging || gap == 0
            || after.percent >= before.percent.saturating_add(RECHARGE_JUMP)
        {
            continue;
        }
        let delta = before.percent as i64 - after.percent as i64;
        let weight = 0.5_f64.powf(newest.saturating_sub(after.at) as f64 / RECENCY_HALF_LIFE);
        if gap <= ACTIVE_GAP {
            evidence += delta;
            active_drop += weight * delta as f64;
            active_seconds += weight * gap as f64;
        }
        if gap <= CALENDAR_GAP {
            calendar_drop += weight * delta as f64;
            calendar_seconds += weight * gap as f64;
            calendar_span += gap;
        }
    }
    let confidence = match evidence {
        drop if drop < 5 => return None,
        drop if drop < 20 => Confidence::Low,
        drop if drop < 50 => Confidence::Medium,
        _ => Confidence::High,
    };
    if active_drop <= 0.0 {
        return None;
    }
    let active_hours = (100.0 * active_seconds / 3600.0 / active_drop).clamp(5.0, 1000.0);
    let calendar_days = (calendar_drop > 0.0 && calendar_span >= 24 * 60 * 60)
        .then(|| (100.0 * calendar_seconds / 86_400.0 / calendar_drop).clamp(0.5, 365.0));
    Some(Estimate { active_hours, calendar_days, confidence })
}

const LOW_BATTERY_LEVELS: [u8; 2] = [20, 5];
/// A warning re-arms only after charging or a real recovery above this level,
/// so ±1% jitter around a threshold never repeats the notification.
const LOW_BATTERY_REARM: u8 = 25;

/// Returns the threshold to announce now (if any) and the new "already
/// announced" state. Each threshold is announced once per discharge.
fn low_battery_alert(announced: Option<u8>, percent: u8, charging: bool) -> (Option<u8>, Option<u8>) {
    if charging || percent > LOW_BATTERY_REARM {
        return (None, None);
    }
    match LOW_BATTERY_LEVELS.iter().copied().filter(|level| percent <= *level).min() {
        Some(level) if announced.is_none_or(|previous| level < previous) => (Some(level), Some(level)),
        _ => (None, announced),
    }
}

fn low_battery_message(level: u8, percent: u8, learned: Option<Estimate>) -> (&'static str, String) {
    let hours = learned.map(|estimate| format!(" (~{:.0} h de uso)", estimate.active_hours * percent as f64 / 100.0))
        .unwrap_or_default();
    if level <= 5 {
        ("Batería crítica · R5 Ultra", format!("Queda {percent}%{hours}. Conectá el mouse para cargarlo."))
    } else {
        ("Batería baja · R5 Ultra", format!("Queda {percent}%{hours}. Conviene cargarlo pronto."))
    }
}

/// Short Spanish date independent of the Windows display language.
fn spanish_date(date: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    const DAYS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];
    const MONTHS: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"];
    format!("{} {} {}", DAYS[date.weekday().num_days_from_monday() as usize], date.day(), MONTHS[date.month0() as usize])
}

/// The local date by which the mouse should be charged at the user's rhythm.
fn charge_by(learned: Option<Estimate>, percent: u8, now: u64) -> Option<String> {
    let days = remaining_days(learned, percent)?;
    let at = DateTime::from_timestamp((now as f64 + days * 86_400.0) as i64, 0)?;
    Some(spanish_date(at.with_timezone(&chrono::Local).date_naive()))
}

/// A clearly provisional baseline keeps the live autonomy useful before enough
/// discharge has been observed. The adjacent full-charge card remains
/// `Aprendiendo`, so this is never presented as a learned calibration.
fn remaining_hours(learned: Option<Estimate>, percent: u8) -> f64 {
    learned.map_or(200.0, |estimate| estimate.active_hours) * percent as f64 / 100.0
}

fn remaining_days(learned: Option<Estimate>, percent: u8) -> Option<f64> {
    learned?.calendar_days.map(|days| days * percent as f64 / 100.0)
}

fn format_days(days: f64) -> String {
    if days < 1.95 { format!("{days:.1} días").replace('.', ",") } else { format!("{days:.0} días") }
}

fn reading_tip(percent: u8, charging: bool, learned: Option<Estimate>, now: u64) -> String {
    if charging {
        return format!("R5 Ultra · {percent}% · cargando");
    }
    let hours = format!("~{:.0} h de uso", remaining_hours(learned, percent));
    match (learned, remaining_days(learned, percent), charge_by(learned, percent, now)) {
        (Some(_), Some(days), Some(date)) =>
            format!("R5 Ultra · {percent}% · {hours} · ~{} · cargar antes del {date}", format_days(days)),
        (Some(estimate), _, _) => format!("R5 Ultra · {percent}% · {hours} · {}", estimate.confidence.label()),
        (None, _, _) => format!("R5 Ultra · {percent}% · {hours} (estimación inicial)"),
    }
}

#[cfg(test)]
mod tests {
    use super::{advance_deadline, estimate, Estimate, Sample, POLL_INTERVAL};

    #[test]
    fn polling_deadline_does_not_add_query_duration_or_manual_refresh() {
        let start = std::time::Instant::now();
        let next = advance_deadline(start, start + std::time::Duration::from_secs(2));
        assert_eq!(next, start + POLL_INTERVAL);
        assert_eq!(advance_deadline(next, start + std::time::Duration::from_secs(10)), next);
        assert_eq!(advance_deadline(next, start + std::time::Duration::from_secs(75)), start + POLL_INTERVAL * 3);
    }

    #[test]
    fn run_command_quotes_paths_with_spaces_without_literal_escape_characters() {
        let path = std::path::Path::new(r"C:\Users\User Name\R5 Battery Estimator\R5BatteryEstimatorV2.exe");
        assert_eq!(super::autostart_command(path), r#""C:\Users\User Name\R5 Battery Estimator\R5BatteryEstimatorV2.exe""#);
    }

    #[test]
    fn tooltip_distinguishes_valid_telemetry_from_provisional_and_learned_estimates() {
        let now = 1_800_000_000;
        let initial = super::reading_tip(78, false, None, now);
        assert_eq!(initial, "R5 Ultra · 78% · ~156 h de uso (estimación inicial)");
        let estimate = Estimate { active_hours: 100.0, calendar_days: Some(28.0), confidence: super::Confidence::Medium };
        let learned = super::reading_tip(78, false, Some(estimate), now);
        assert!(learned.starts_with("R5 Ultra · 78% · ~78 h de uso · ~22 días · cargar antes del "), "{learned}");
        let without_days = Estimate { calendar_days: None, ..estimate };
        assert_eq!(super::reading_tip(78, false, Some(without_days), now), "R5 Ultra · 78% · ~78 h de uso · confianza media");
        assert_eq!(super::reading_tip(40, true, Some(estimate), now), "R5 Ultra · 40% · cargando");
        for tip in [initial, learned] {
            assert!(tip.encode_utf16().count() < 128);
        }
    }

    #[test]
    fn sleeping_receiver_preserves_only_explicitly_stale_percentage() {
        let sample = Sample { at: 100, percent: 93, charging: false };
        let pending = super::ProbeResult::Error { code: "reading_unavailable", message: String::new() };
        assert_eq!(super::panel_battery(Some(&pending), Some(sample.clone())),
            (Some(93), "Último valor conocido", false));
        assert_eq!(super::panel_battery(Some(&pending), None), (None, "Último valor conocido", false));
        let missing = super::ProbeResult::Error { code: "device_unavailable", message: String::new() };
        assert_eq!(super::panel_battery(Some(&missing), Some(sample.clone())), (None, "Sin lectura", false));
        assert_eq!(super::pending_tip(Some(sample.clone()), 130), "R5 Ultra · 93% hace 30 s · mouse en reposo");
        assert_eq!(super::pending_tip(Some(sample), 100 + 7_200), "R5 Ultra · 93% hace 2 h · mouse en reposo");
        assert!(super::pending_tip(None, 0).encode_utf16().count() < 128);
        let awake = super::ProbeResult::Ok { reading: r5_battery_estimator::battery::protocol::ParsedBattery {
            percent: 92, charging: false,
            layout: r5_battery_estimator::battery::protocol::ReportLayout::Shifted,
        } };
        assert_eq!(super::panel_battery(Some(&awake), None), (Some(92), "En batería", true));
    }

    #[test]
    fn elapsed_labels_round_to_readable_units() {
        assert_eq!(super::elapsed_label(45), "45 s");
        assert_eq!(super::elapsed_label(600), "10 min");
        assert_eq!(super::elapsed_label(3 * 3_600 + 600), "3 h");
    }

    #[test]
    fn recent_drain_outweighs_old_drain() {
        // Three weeks at 600 s per point, then one week at 300 s per point.
        let mut samples = Vec::new();
        let mut at = 0;
        let mut percent: u8 = 100;
        for (days, spacing) in [(21_u64, 600_u64), (7, 300)] {
            let end = at + days * 86_400;
            while at < end {
                // One awake hour per day, then the PC sleeps until the next day.
                let session_end = at + 3_600;
                samples.push(Sample { at, percent, charging: false });
                while at + spacing <= session_end {
                    at += spacing;
                    percent -= 1;
                    samples.push(Sample { at, percent, charging: false });
                    if percent == 10 {
                        percent = 100;
                        at += 600;
                        samples.push(Sample { at, percent, charging: true });
                    }
                }
                at = session_end - (session_end % 86_400) + 86_400;
            }
        }
        let learned = estimate(&samples).unwrap();
        let old_rate_hours = 600.0 * 100.0 / 3600.0;
        let new_rate_hours = 300.0 * 100.0 / 3600.0;
        let unweighted = (old_rate_hours + new_rate_hours) / 2.0;
        assert!(learned.active_hours < unweighted, "{} h", learned.active_hours);
        assert!(learned.active_hours > new_rate_hours);
    }

    #[test]
    fn low_battery_announces_each_threshold_once_per_discharge() {
        use super::low_battery_alert as alert;
        assert_eq!(alert(None, 40, false), (None, None));
        assert_eq!(alert(None, 20, false), (Some(20), Some(20)));
        assert_eq!(alert(Some(20), 19, false), (None, Some(20)));
        assert_eq!(alert(Some(20), 21, false), (None, Some(20)), "jitter above 20% does not re-arm");
        assert_eq!(alert(Some(20), 5, false), (Some(5), Some(5)));
        assert_eq!(alert(Some(5), 4, false), (None, Some(5)));
        assert_eq!(alert(Some(5), 4, true), (None, None), "charging re-arms");
        assert_eq!(alert(Some(20), 30, false), (None, None), "recovery above 25% re-arms");
        assert_eq!(alert(None, 3, false), (Some(5), Some(5)), "starting below 5% only sends the critical warning");
        let (title, body) = super::low_battery_message(20, 19, Some(Estimate {
            active_hours: 70.0, calendar_days: None, confidence: super::Confidence::High,
        }));
        assert_eq!(title, "Batería baja · R5 Ultra");
        assert_eq!(body, "Queda 19% (~13 h de uso). Conviene cargarlo pronto.");
        assert!(super::low_battery_message(5, 4, None).0.contains("crítica"));
    }

    #[test]
    fn spanish_dates_do_not_depend_on_windows_language() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(super::spanish_date(date), "jue 5 nov");
        let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
        assert_eq!(super::spanish_date(date), "dom 13 sept");
    }

    #[test]
    fn chart_averages_hours_and_finds_day_boundaries() {
        let samples = [
            Sample { at: 100, percent: 50, charging: false },
            Sample { at: 200, percent: 51, charging: false },
            Sample { at: 3_700, percent: 49, charging: false },
            Sample { at: 99_999, percent: 1, charging: false },
        ];
        let hourly = super::hourly_average(&samples, 0, 7_200);
        assert_eq!(hourly, vec![(1_800, 50.5), (5_400, 49.0)]);
        let now = 1_800_000_000;
        let midnights = super::local_midnights(now - 7 * 86_400, now);
        assert!((6..=8).contains(&midnights.len()));
        assert!(midnights.windows(2).all(|pair| pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1));
    }

    #[test]
    fn dpi_fit_preserves_aspect_and_click_targets_in_laptop_work_areas() {
        for (dpi, width, height) in [(96, 1342, 704), (144, 1896, 1016), (192, 2536, 1376), (96, 500, 300)] {
            let (fitted_width, fitted_height) = super::fitted_size(dpi, width, height);
            assert!(fitted_width <= width && fitted_height <= height);
            let x_scale = fitted_width as f64 / super::WIDTH as f64;
            let y_scale = fitted_height as f64 / super::HEIGHT as f64;
            assert!((x_scale - y_scale).abs() < 0.002);
            for button in [super::REFRESH, super::MINIMIZE, super::CLOSE] {
                let x = ((button.x + button.w / 2) as f64 * x_scale).round() as i32;
                let y = ((button.y + button.h / 2) as f64 * y_scale).round() as i32;
                let logical = super::logical_point(x, y, fitted_width, fitted_height);
                assert!(button.contains(logical.0, logical.1));
            }
            assert!(!super::REFRESH.contains(-1, -1));
        }
    }

    /// Recency weighting shifts mixed-rate fixtures slightly from the plain average.
    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= expected * 0.03
    }

    fn cycle(at: u64, spacing: u64) -> Vec<Sample> {
        (5..=100).rev().enumerate().map(|(index, percent)| Sample {
            at: at + index as u64 * spacing, percent, charging: false,
        }).collect()
    }

    #[test]
    fn loaded_history_repairs_order_duplicates_ranges_and_dates() {
        let now = 10_000_000;
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
    fn same_second_transitions_survive_normalization_as_charging_boundary() {
        let mut samples = cycle(100, 300);
        let index = 50;
        let original = samples[index].clone();
        samples.insert(index + 1, Sample { at: original.at, percent: original.percent, charging: true });
        let normalized = super::normalize_history(samples, 40_000);
        assert_eq!(normalized.len(), 97);
        assert_eq!(normalized[index], original);
        assert!(normalized[index + 1].charging);
        // Both neighbours of the charging sample are skipped; the rest still teaches 300 s per point.
        let learned = estimate(&normalized).unwrap();
        assert!((learned.active_hours - 300.0 * 100.0 / 3600.0).abs() < 0.001);
    }

    #[test]
    fn learns_from_partial_discharges_split_by_sleep_and_pc_off() {
        // Mirrors the real history: short awake sessions, nights off, never near 0% or 100%.
        let mut samples = Vec::new();
        let mut percent = 94;
        for day in 0..14_u64 {
            for session in 0..2_u64 {
                // Two hours of 5-minute readings losing one point every 40 minutes,
                // then 30 minutes asleep before the next session.
                let start = day * 86_400 + session * 9_000;
                for step in 0..=24_u64 {
                    samples.push(Sample { at: start + step * 300, percent: percent - (step / 8) as u8, charging: false });
                }
                percent -= 3;
            }
            // The PC stays off for the rest of the day.
        }
        let learned = estimate(&samples).expect("partial discharges are enough to learn");
        assert!(close(learned.active_hours, 2_400.0 * 100.0 / 3600.0), "{} h", learned.active_hours);
        assert_eq!(learned.confidence, super::Confidence::High);
        let days = learned.calendar_days.unwrap();
        assert!(days > 10.0 && days < 25.0, "calendar days {days}");
    }

    #[test]
    fn one_percent_jitter_cancels_instead_of_breaking_learning() {
        let mut samples = Vec::new();
        for index in 0..60_u64 {
            // 80, 79, 80, 79, 78, 79 ... net one point every 3 readings.
            let percent = 80 - (index / 3) as u8 + u8::from(index % 3 == 1);
            samples.push(Sample { at: index * 300, percent, charging: false });
        }
        let learned = estimate(&samples).unwrap();
        let drop = samples[0].percent - samples[59].percent;
        assert!(close(learned.active_hours, 59.0 * 300.0 * 100.0 / 3600.0 / drop as f64), "{} h", learned.active_hours);
    }

    #[test]
    fn recharge_jump_without_charging_flag_is_a_boundary() {
        let mut samples = cycle(0, 300);
        samples.truncate(20); // 100 → 81
        // Cable charge while the app could not read: next reading is 95% an hour later.
        samples.push(Sample { at: 20 * 300 + 3_600, percent: 95, charging: false });
        samples.extend(cycle(20 * 300 + 3_900, 300).into_iter().skip(5).take(10)); // 95 → 86
        let learned = estimate(&samples).unwrap();
        // 19 + 9 points, all at 300 s each; the +14% jump adds neither drop nor time.
        assert!((learned.active_hours - 300.0 * 100.0 / 3600.0).abs() < 0.001);
    }

    #[test]
    fn gaps_count_for_calendar_but_not_for_active_use() {
        let mut samples = Vec::new();
        for (start, first) in [(0_u64, 90_u8), (3_000 + 86_400, 83)] {
            for step in 0..=5_u64 {
                samples.push(Sample { at: start + step * 600, percent: first - step as u8, charging: false });
            }
        }
        let learned = estimate(&samples).unwrap();
        // 10 points over 6000 active seconds; the day-long gap only adds calendar time.
        assert!((learned.active_hours - 6_000.0 * 100.0 / 3600.0 / 10.0).abs() < 0.001);
        let days = learned.calendar_days.unwrap();
        // ~8.9 days unweighted; the recent half weighs slightly more. Active-only would be ~0.7.
        assert!((8.5..9.5).contains(&days), "{days} days");
        assert_eq!(learned.confidence, super::Confidence::Low);
    }

    #[test]
    fn does_not_claim_learning_below_five_points() {
        let samples = [
            Sample { at: 0, percent: 80, charging: false },
            Sample { at: 300, percent: 79, charging: false },
            Sample { at: 600, percent: 77, charging: false },
            Sample { at: 900, percent: 76, charging: false },
        ];
        assert_eq!(estimate(&samples), None);
        assert_eq!(estimate(&[]), None);
    }

    #[test]
    fn charging_segments_teach_nothing() {
        let samples: Vec<_> = (0..40_u64).map(|index| Sample {
            at: index * 300, percent: 100 - index as u8, charging: true,
        }).collect();
        assert_eq!(estimate(&samples), None);
    }

    #[test]
    fn confidence_grows_with_observed_drop() {
        for (points, expected) in [(10, super::Confidence::Low), (30, super::Confidence::Medium), (60, super::Confidence::High)] {
            let mut samples = cycle(0, 300);
            samples.truncate(points + 1);
            assert_eq!(estimate(&samples).unwrap().confidence, expected, "{points} points");
        }
    }

    #[test]
    fn plateau_compaction_keeps_one_reading_per_active_interval_and_preserves_gaps() {
        let mut items = Vec::new();
        for index in 0..=40_u64 {
            super::append_sample(&mut items, Sample { at: index * 30, percent: 70, charging: false });
        }
        // 20 minutes of 30 s polls at one percentage: pairs never span more than ACTIVE_GAP.
        assert!(items.len() <= 4, "{} samples", items.len());
        assert!(items.windows(2).all(|pair| pair[1].at - pair[0].at <= super::ACTIVE_GAP));
        assert_eq!(items.last().unwrap().at, 1_200);
        super::append_sample(&mut items, Sample { at: 1_200 + 3_600, percent: 70, charging: false });
        assert_eq!(items.last().unwrap().at, 4_800);
        assert_eq!(items[items.len() - 2].at, 1_200, "sleep gap stays visible");
        super::append_sample(&mut items, Sample { at: 4_830, percent: 69, charging: false });
        assert_eq!(items.last().unwrap().percent, 69);
    }

    #[test]
    #[ignore = "reads this PC's local battery history"]
    fn local_history_produces_an_estimate() {
        let samples = super::load_history();
        let learned = estimate(&samples);
        println!("{} samples -> {learned:?}", samples.len());
        if let Some(last) = samples.last() {
            println!("{}% -> {}", last.percent, super::reading_tip(last.percent, last.charging, learned, super::now_seconds()));
        }
        assert!(learned.is_some());
    }

    #[test]
    fn history_keeps_sixty_days() {
        let now = 100 * 86_400;
        let samples = vec![
            Sample { at: now - 61 * 86_400, percent: 90, charging: false },
            Sample { at: now - 59 * 86_400, percent: 80, charging: false },
        ];
        let kept = super::normalize_history(samples, now);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].percent, 80);
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
            let (base, highlight, glow, _) = level_palette(value);
            let sweep = value as f32 * 3.6;
            // A raster mask is actually blurred with the GDI+ Blur effect before composition.
            draw_blurred_arc(dc, x, y, size, left - x, top - y, diameter, sweep, glow);
            // The shaded base plus the narrow highlight form one illuminated,
            // three-dimensional progress surface.  They are inside the real blurred
            // mask below, so this is not a detached shadow or a third ring.
            draw_gp_arc(
                graphics,
                left,
                top,
                diameter,
                base,
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
                highlight,
                15.0,
                -90.0,
                sweep.min(359.95),
                value < 100,
            );
            if value == 100 {
                draw_gp_arc(
                    graphics, left, top, diameter, base, 22.0, -90.0, 359.95, false,
                );
                draw_gp_arc(
                    graphics, left, top, diameter, highlight, 15.0, -90.0, 359.95, false,
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
    battery_glyph(dc, center_x, y + 173, percent.map(|value| level_palette(value).3));
    draw_center(dc, status, x + 48, y + 202, 210, 28, 15, MUTED, false);
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
    glow: u32,
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
            glow,
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
    color: Option<u32>,
) {
    let pen = CreatePen(PS_SOLID, 3, rgb(color.unwrap_or(0x7890A4)));
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
        .map(|samples| estimate(&samples));
    match BATTERY
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.clone())
    {
        Some(ProbeResult::Ok { reading }) =>
            reading_tip(reading.percent, reading.charging, learned.flatten(), now_seconds()),
        Some(ProbeResult::Error { code: "reading_unavailable", .. }) => pending_tip(last_valid_sample(), now_seconds()),
        _ => "R5 Ultra · receptor no detectado".to_owned(),
    }
}

thread_local! {
    /// Lowest low-battery threshold already announced in this discharge.
    static LOW_BATTERY_ANNOUNCED: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) };
}

/// Runs on the UI thread after every reading and raises a Windows
/// notification from the tray icon when a low-battery threshold is crossed.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn check_low_battery(hwnd: HWND) {
    let Some(ProbeResult::Ok { reading }) = BATTERY.get().and_then(|state| state.lock().ok()).map(|state| state.clone())
    else {
        return;
    };
    let (alert, announced) = low_battery_alert(LOW_BATTERY_ANNOUNCED.get(), reading.percent, reading.charging);
    LOW_BATTERY_ANNOUNCED.set(announced);
    let Some(level) = alert else { return };
    let learned = HISTORY.get().and_then(|history| history.lock().ok()).and_then(|samples| estimate(&samples));
    let (title, body) = low_battery_message(level, reading.percent, learned);
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_WARNING,
        ..Default::default()
    };
    let title: Vec<u16> = title.encode_utf16().take(data.szInfoTitle.len() - 1).collect();
    data.szInfoTitle[..title.len()].copy_from_slice(&title);
    let body: Vec<u16> = body.encode_utf16().take(data.szInfo.len() - 1).collect();
    data.szInfo[..body.len()].copy_from_slice(&body);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
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
