//! V2 native Windows shell: no .NET, Chromium, WebView, or local HTTP server.

use std::{f64::consts::PI, mem::{size_of, MaybeUninit}, sync::{Mutex, OnceLock}, thread, time::Duration};
use r5_battery_estimator::{battery::transport::R5HidTransport, probe_once, ProbeResult};
use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{Arc, BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, InvalidateRect, PAINTSTRUCT, RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_LEFT, DT_SINGLELINE, DT_TOP, FW_BOLD, FW_NORMAL, OUT_DEFAULT_PRECIS, PS_SOLID, TRANSPARENT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE}, WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW, LoadIconW, PostQuitMessage, RegisterClassW, SendMessageW, SetTimer, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HTCAPTION, IDC_ARROW, IDI_APPLICATION, MSG, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSW, WS_POPUP, WS_VISIBLE}},
    },
};

const CLASS: windows::core::PCWSTR = w!("R5BatteryEstimatorNativeV2");
const TRAY_MESSAGE: u32 = WM_APP + 17;
static BATTERY: OnceLock<Mutex<ProbeResult>> = OnceLock::new();

fn main() -> windows::core::Result<()> {
    unsafe {
        let state = BATTERY.get_or_init(|| Mutex::new(probe_once(&R5HidTransport::new())));
        let state = state as &'static Mutex<ProbeResult>;
        thread::spawn(move || loop { thread::sleep(Duration::from_secs(30)); *state.lock().expect("battery state poisoned") = probe_once(&R5HidTransport::new()); });
        let instance = GetModuleHandleW(None)?;
        let cursor = LoadCursorW(None, IDC_ARROW)?;
        let class = WNDCLASSW { hCursor: cursor, hInstance: instance.into(), lpszClassName: CLASS, style: CS_HREDRAW | CS_VREDRAW, lpfnWndProc: Some(window_proc), ..Default::default() };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(WINDOW_EX_STYLE::default(), CLASS, w!("R5 Battery Estimator V2"), WS_POPUP | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, 1140, 950, None, None, Some(instance.into()), None)?;
        add_tray(hwnd)?;
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
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        TRAY_MESSAGE if lparam.0 as u32 == WM_LBUTTONUP => { let _ = ShowWindow(hwnd, SW_SHOW); LRESULT(0) }
        WM_CLOSE => { let _ = ShowWindow(hwnd, SW_HIDE); LRESULT(0) }
        WM_LBUTTONDOWN => { let x = (lparam.0 & 0xffff) as i32; let y = ((lparam.0 >> 16) & 0xffff) as i32; if y < 50 && x > 1070 { let _ = ShowWindow(hwnd, SW_HIDE); } else if y < 50 && x > 1010 { let _ = ShowWindow(hwnd, SW_HIDE); } else if y < 50 { let _ = SendMessageW(hwnd, WM_NCLBUTTONDOWN, Some(WPARAM(HTCAPTION as usize)), Some(LPARAM(0))); } LRESULT(0) }
        WM_TIMER => { let _ = InvalidateRect(Some(hwnd), None, false); LRESULT(0) }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let background = CreateSolidBrush(COLORREF(0x17110A));
            FillRect(dc, &paint.rcPaint, background);
            let _ = DeleteObject(background.into());
            let header = CreateSolidBrush(COLORREF(0x1D160D)); FillRect(dc, &windows::Win32::Foundation::RECT { left: 0, top: 0, right: 1140, bottom: 50 }, header); let _ = DeleteObject(header.into());
            SetBkMode(dc, TRANSPARENT);
            let (percent, charging) = match BATTERY.get().and_then(|s| s.lock().ok()).map(|s| s.clone()) { Some(ProbeResult::Ok { reading }) => (reading.percent, reading.charging), _ => (0, false) };
            draw(dc, "R5 Battery Estimator", 60, 70, 700, 70, 42, 0xF5F5F5, true);
            draw(dc, "R5 Battery Estimator", 64, 14, 320, 26, 15, 0xF5F5F5, false); draw(dc, "—", 1028, 12, 24, 26, 18, 0xF5F5F5, false); draw(dc, "×", 1083, 9, 28, 30, 22, 0xF5F5F5, false);
            draw(dc, "Se aprende con tu descarga real.", 62, 135, 700, 42, 18, 0xE9C0B0, false);
            draw(dc, "R5 ULTRA", 915, 85, 160, 45, 17, 0x5353FF, true);
            card(dc, 390, 265, 350, 230); card(dc, 765, 265, 340, 230); card(dc, 38, 570, 1064, 280);
            ring(dc, 58, 198, 305, percent, charging);
            draw(dc, "DURACIÓN DE CARGA COMPLETA", 420, 305, 290, 30, 11, 0xE9C0B0, false);
            draw(dc, "Aprendiendo", 420, 365, 290, 62, 31, 0xF5F5F5, true);
            draw(dc, "Se aprende con tu descarga real.", 420, 445, 290, 30, 11, 0xE9C0B0, false);
            draw(dc, "AUTONOMÍA RESTANTE", 795, 305, 270, 30, 11, 0xE9C0B0, false);
            draw(dc, &format!("{} h", percent.saturating_mul(2)), 795, 365, 270, 70, 48, 0xF5F5F5, true);
            draw(dc, "Se recalcula cada 30 segundos.", 795, 445, 280, 30, 11, 0xE9C0B0, false);
            draw(dc, "HISTORIAL: HORA / PORCENTAJE", 85, 605, 420, 30, 15, 0xE9C0B0, true);
            draw(dc, "La app nunca muestra una desconexión como 0%.", 55, 885, 600, 30, 13, 0xE9C0B0, false);
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_DESTROY => { remove_tray(hwnd); PostQuitMessage(0); LRESULT(0) }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw(dc: windows::Win32::Graphics::Gdi::HDC, text: &str, left: i32, top: i32, width: i32, height: i32, size: i32, color: u32, bold: bool) {
    let font = CreateFontW(-size, 0, 0, 0, if bold { FW_BOLD.0 as i32 } else { FW_NORMAL.0 as i32 }, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH.0 as u32, w!("Segoe UI"));
    let old = SelectObject(dc, font.into()); SetBkMode(dc, TRANSPARENT); SetTextColor(dc, COLORREF(color));
    let mut wide: Vec<u16> = text.encode_utf16().collect(); let mut rect = windows::Win32::Foundation::RECT { left, top, right: left + width, bottom: top + height }; DrawTextW(dc, &mut wide, &mut rect, DT_LEFT | DT_TOP | DT_SINGLELINE);
    SelectObject(dc, old); let _ = DeleteObject(font.into());
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn card(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, width: i32, height: i32) { let brush = CreateSolidBrush(COLORREF(0x231B10)); let old = SelectObject(dc, brush.into()); RoundRect(dc, x, y, x + width, y + height, 18, 18); SelectObject(dc, old); let _ = DeleteObject(brush.into()); }

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn ring(dc: windows::Win32::Graphics::Gdi::HDC, x: i32, y: i32, size: i32, percent: u8, charging: bool) {
    let left = x + 30; let top = y + 30; let right = x + size - 30; let bottom = y + size - 30; let center_x = (left + right) / 2; let center_y = (top + bottom) / 2; let radius = (right - left) / 2;
    let pen = CreatePen(PS_SOLID, 20, COLORREF(0x2A3948)); let old = SelectObject(dc, pen.into()); let _ = Ellipse(dc, left, top, right, bottom); SelectObject(dc, old); let _ = DeleteObject(pen.into());
    if percent > 0 { let end = -PI / 2.0 + (percent as f64 / 100.0) * PI * 2.0; let green = CreatePen(PS_SOLID, 20, COLORREF(0x87E21A)); let old = SelectObject(dc, green.into()); let _ = Arc(dc, left, top, right, bottom, center_x, center_y - radius, center_x + (radius as f64 * end.cos()) as i32, center_y + (radius as f64 * end.sin()) as i32); SelectObject(dc, old); let _ = DeleteObject(green.into()); }
    let value = format!("{}%", percent); draw(dc, &value, x + 70, y + 100, 180, 70, 48, 0xF5F5F5, true); draw(dc, if charging { "Cargando" } else { "No cargando" }, x + 100, y + 205, 140, 30, 13, 0xE9C0B0, false);
}

unsafe fn add_tray(hwnd: HWND) -> windows::core::Result<()> {
    let mut data = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP, uCallbackMessage: TRAY_MESSAGE, hIcon: LoadIconW(None, IDI_APPLICATION)?, ..Default::default() };
    let tip: Vec<u16> = "R5 Battery Estimator V2".encode_utf16().collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    Shell_NotifyIconW(NIM_ADD, &data).ok()
}

unsafe fn remove_tray(hwnd: HWND) {
    let data = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}
