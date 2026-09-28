using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Drawing.Drawing2D;
using Forms = System.Windows.Forms;
using Drawing = System.Drawing;

namespace R5BatteryEstimator.Native;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        Forms.Application.EnableVisualStyles();
        Forms.Application.SetCompatibleTextRenderingDefault(false);
        Forms.Application.Run(new BatteryForm());
    }
}

internal sealed class BatteryForm : Forms.Form
{
    private readonly Forms.Label _percent = new() { Text = "—%", Font = new Drawing.Font("Segoe UI", 32, Drawing.FontStyle.Bold), AutoSize = false, TextAlign = Drawing.ContentAlignment.MiddleCenter, Dock = Forms.DockStyle.Fill, ForeColor = Drawing.Color.White };
    private readonly Forms.Label _status = new() { Text = "Leyendo el receptor…", AutoSize = true, ForeColor = Drawing.Color.FromArgb(185, 190, 200) };
    private readonly Forms.Label _estimate = new() { Text = "—", Font = new Drawing.Font("Segoe UI", 22, Drawing.FontStyle.Bold), AutoSize = true, ForeColor = Drawing.Color.White };
    private readonly Forms.Label _remaining = new() { Text = "—", Font = new Drawing.Font("Segoe UI", 22, Drawing.FontStyle.Bold), AutoSize = true, ForeColor = Drawing.Color.White };
    private readonly Forms.Label _updated = new() { Text = "Sin datos", Font = new Drawing.Font("Segoe UI", 16, Drawing.FontStyle.Bold), AutoSize = true, ForeColor = Drawing.Color.White };
    private readonly Forms.Panel _circle = new() { Size = new Drawing.Size(142, 142), BackColor = Drawing.Color.DimGray };
    private readonly StatusRing _ring = new() { Location = new Drawing.Point(45, 210) };
    private readonly BatteryHistory _history = new();
    private readonly BatteryChart _chart = new() { Location = new Drawing.Point(30, 392), Size = new Drawing.Size(600, 135) };
    private readonly Forms.Timer _timer = new() { Interval = 30_000 };
    private readonly Forms.NotifyIcon _tray;
    private readonly Drawing.Image _mascot;
    private readonly Drawing.Image _taskbarMascot;
    private bool _exitRequested;

    public BatteryForm()
    {
        Text = "R5 Battery Estimator";
        ClientSize = new Drawing.Size(1120, 1010);
        MinimumSize = new Drawing.Size(940, 840);
        FormBorderStyle = Forms.FormBorderStyle.None;
        BackColor = Drawing.Color.FromArgb(10, 17, 23);
        ForeColor = Drawing.Color.White;
        StartPosition = Forms.FormStartPosition.CenterScreen;
        Paint += (_, e) =>
        {
            using var glow = new Drawing.Drawing2D.PathGradientBrush(new[] { new Drawing.Point(180, 365), new Drawing.Point(430, 365), new Drawing.Point(305, 580) }) { CenterColor = Drawing.Color.FromArgb(30, 33, 226, 139), SurroundColors = [Drawing.Color.FromArgb(0, 10, 17, 23), Drawing.Color.FromArgb(0, 10, 17, 23), Drawing.Color.FromArgb(0, 10, 17, 23)] };
            e.Graphics.FillEllipse(glow, 25, 190, 380, 380);
        };
        Resize += (_, _) => SetRoundedRegion();

        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add("Abrir panel", null, (_, _) => ShowPanel());
        menu.Items.Add("Actualizar perfil", null, (_, _) => MarkProfileChecked());
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add("Cerrar programa", null, (_, _) => ExitProgram());
        _mascot = Drawing.Image.FromFile(Path.Combine(AppContext.BaseDirectory, "Assets", "shark-battery.png"));
        _taskbarMascot = Drawing.Image.FromFile(Path.Combine(AppContext.BaseDirectory, "Assets", "shark-head-battery.png"));
        Icon = CreateAppIcon();
        _tray = new Forms.NotifyIcon { Icon = CreateTrayIcon(Drawing.Color.DodgerBlue), Text = "R5 Battery Estimator", ContextMenuStrip = menu, Visible = true };
        _tray.MouseUp += (_, e) => { if (e.Button == Forms.MouseButtons.Left) menu.Show(Forms.Cursor.Position); };

        BuildLayout();
        _chart.Items = _history.Recent(TimeSpan.FromHours(24));
        Load += async (_, _) => await RefreshAsync();
        _timer.Tick += async (_, _) => await RefreshAsync();
        _timer.Start();
        Resize += (_, _) => { if (WindowState == Forms.FormWindowState.Minimized) Hide(); };
        FormClosing += (_, e) => { if (!_exitRequested) { e.Cancel = true; Hide(); } };
    }

    private void BuildLayout()
    {
        var chrome = new Forms.Panel { Dock = Forms.DockStyle.Top, Height = 60, BackColor = Drawing.Color.FromArgb(13, 22, 29) };
        chrome.Paint += (_, e) => { using var p = new Drawing.Pen(Drawing.Color.FromArgb(35, 49, 59)); e.Graphics.DrawLine(p, 0, chrome.Height - 1, chrome.Width, chrome.Height - 1); };
        chrome.MouseDown += DragWindow;
        var tinyLogo = new Forms.PictureBox { Image = _mascot, Location = new Drawing.Point(20, 14), Size = new Drawing.Size(31, 31), SizeMode = Forms.PictureBoxSizeMode.Zoom };
        var chromeTitle = new Forms.Label { Text = "R5 Battery Estimator", Location = new Drawing.Point(66, 17), AutoSize = true, Font = new Drawing.Font("Segoe UI", 15), ForeColor = Drawing.Color.FromArgb(235, 240, 247) };
        var minimize = ChromeButton("—", 974, (_, _) => WindowState = Forms.FormWindowState.Minimized);
        var maximize = ChromeButton("□", 1022, (_, _) => WindowState = WindowState == Forms.FormWindowState.Maximized ? Forms.FormWindowState.Normal : Forms.FormWindowState.Maximized);
        var close = ChromeButton("×", 1070, (_, _) => Hide());
        foreach (var button in new[] { minimize, maximize, close }) { button.Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Right; chrome.Controls.Add(button); }
        chrome.Controls.AddRange([tinyLogo, chromeTitle]);
        var title = new Forms.Label { Text = "R5 Battery Estimator", Font = new Drawing.Font("Segoe UI", 40, Drawing.FontStyle.Bold), AutoSize = true, Location = new Drawing.Point(60, 98), ForeColor = Drawing.Color.FromArgb(247, 249, 252) };
        var subtitle = new Forms.Label { Text = "Se aprende con tu descarga real.", Font = new Drawing.Font("Segoe UI", 18), AutoSize = true, Location = new Drawing.Point(62, 163), ForeColor = Drawing.Color.FromArgb(188, 207, 233) };
        var badge = new Forms.Label { Text = "R5 ULTRA", AutoSize = true, Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Right, Location = new Drawing.Point(875, 111), ForeColor = Drawing.Color.FromArgb(255, 83, 83), Font = new Drawing.Font("Segoe UI", 16, Drawing.FontStyle.Bold) };
        var panelLogo = new Forms.PictureBox { Image = _mascot, Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Right, Location = new Drawing.Point(1000, 82), Size = new Drawing.Size(84, 84), SizeMode = Forms.PictureBoxSizeMode.Zoom };
        _status.Visible = false;
        _ring.Location = new Drawing.Point(42, 200); _ring.Size = new Drawing.Size(345, 345);
        var left = Card("◷   DURACIÓN DE CARGA COMPLETA", _estimate, "Se aprende con tu descarga real.", 405);
        var right = Card("▮▮   AUTONOMÍA RESTANTE", _remaining, "Se recalcula cada 30 segundos.", 755);
        left.Top = right.Top = 285; left.Size = new Drawing.Size(330, 230); right.Size = new Drawing.Size(330, 230);
        left.Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Right; right.Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Right;
        _estimate.Font = new Drawing.Font("Segoe UI", 34, Drawing.FontStyle.Bold); _remaining.Font = new Drawing.Font("Segoe UI", 48, Drawing.FontStyle.Bold);
        _chart.Location = new Drawing.Point(38, 590); _chart.Size = new Drawing.Size(1044, 280); _chart.Anchor = Forms.AnchorStyles.Top | Forms.AnchorStyles.Left | Forms.AnchorStyles.Right;
        var hint = new Forms.Label { Text = "ⓘ   La app nunca muestra una desconexión como 0%.", AutoSize = true, Location = new Drawing.Point(50, 915), Anchor = Forms.AnchorStyles.Left | Forms.AnchorStyles.Bottom, ForeColor = Drawing.Color.FromArgb(190, 208, 233), Font = new Drawing.Font("Segoe UI", 13) };
        var update = new Forms.Button { Text = "⟳   Actualizar ahora", Size = new Drawing.Size(225, 58), Location = new Drawing.Point(855, 895), Anchor = Forms.AnchorStyles.Right | Forms.AnchorStyles.Bottom, BackColor = Drawing.Color.FromArgb(225, 54, 60), ForeColor = Drawing.Color.White, FlatStyle = Forms.FlatStyle.Flat, Font = new Drawing.Font("Segoe UI", 12) };
        update.FlatAppearance.BorderSize = 0; update.Click += async (_, _) => await RefreshAsync();
        Controls.AddRange([chrome, title, subtitle, badge, panelLogo, _ring, left, right, _chart, hint, update]);
        SetRoundedRegion();
    }

    private Forms.Panel Card(string title, Forms.Label value, string note, int left)
    {
        var card = new RoundedPanel { Location = new Drawing.Point(left, 250), Size = new Drawing.Size(295, 92), BackColor = Drawing.Color.FromArgb(16, 27, 35) };
        card.Controls.Add(new Forms.Label { Text = title, AutoSize = true, Location = new Drawing.Point(25, 37), ForeColor = Drawing.Color.FromArgb(199, 216, 241), Font = new Drawing.Font("Segoe UI", 11) });
        value.Location = new Drawing.Point(25, 91); card.Controls.Add(value);
        card.Controls.Add(new Forms.Label { Text = note, AutoSize = true, Location = new Drawing.Point(25, 185), ForeColor = Drawing.Color.FromArgb(190, 208, 233), Font = new Drawing.Font("Segoe UI", 11) });
        return card;
    }

    private Forms.Label ChromeButton(string text, int left, Forms.MouseEventHandler action) { var button = new Forms.Label { Text = text, Location = new Drawing.Point(left, 8), Size = new Drawing.Size(44, 44), TextAlign = Drawing.ContentAlignment.MiddleCenter, ForeColor = Drawing.Color.FromArgb(220, 230, 240), Font = new Drawing.Font("Segoe UI", 22) }; button.MouseDown += action; return button; }
    private void DragWindow(object? sender, Forms.MouseEventArgs e) { if (e.Button == Forms.MouseButtons.Left) { ReleaseCapture(); SendMessage(Handle, 0xA1, (IntPtr)2, IntPtr.Zero); } }
    private void SetRoundedRegion() { using var path = new GraphicsPath(); path.AddArc(0, 0, 24, 24, 180, 90); path.AddArc(Width - 25, 0, 24, 24, 270, 90); path.AddArc(Width - 25, Height - 25, 24, 24, 0, 90); path.AddArc(0, Height - 25, 24, 24, 90, 90); path.CloseFigure(); Region = new Region(path); }
    [DllImport("user32.dll")] private static extern bool ReleaseCapture();
    [DllImport("user32.dll")] private static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);

    private async Task RefreshAsync()
    {
        _status.Text = "Leyendo el receptor…";
        var result = await ProbeRunner.ReadAsync();
        if (result.Percent is not int percent) { _percent.Text = "—%"; _estimate.Text = _history.LearnedFullChargeHours() is double previous ? $"{previous:0} h" : "Aprendiendo"; _remaining.Text = "—"; _updated.Text = "Sin datos"; _status.Text = result.Message; SetTrayText("R5 Battery Estimator — sin lectura válida"); SetTrayColor(Drawing.Color.DimGray); return; }
        var hours = Math.Round(percent * 2.0);
        _history.Add(percent, result.Charging);
        _chart.Items = _history.Recent(TimeSpan.FromHours(24));
        var fullChargeHours = _history.LearnedFullChargeHours() ?? 200;
        _ring.Percent = percent; _ring.Status = result.Charging ? "Cargando" : "No cargando"; _ring.Invalidate(); _percent.Text = $"{percent}%"; _estimate.Text = _history.LearnedFullChargeHours() is double learned ? $"{learned:0} h" : "Aprendiendo"; _remaining.Text = $"{Math.Round(percent * fullChargeHours / 100):0} h"; _updated.Text = DateTime.Now.ToString("HH:mm"); _status.Text = result.Charging ? "Cargando" : "No cargando";
        SetTrayColor(percent > 50 ? Drawing.Color.FromArgb(46, 204, 113) : percent > 20 ? Drawing.Color.FromArgb(241, 196, 15) : Drawing.Color.FromArgb(231, 76, 60));
        SetTrayText($"R5: {percent}% — hasta {Math.Round(percent * fullChargeHours / 100):0} h");
    }

    private void MarkProfileChecked() { _updated.Text = DateTime.Now.ToString("HH:mm"); _status.Text = "Perfil 1 revisado. Se actualizará al detectar cambios del mouse."; }
    private void SetTrayText(string text) => _tray.Text = text.Length <= 63 ? text : text[..63];
    private void SetTrayColor(Drawing.Color color)
    {
        _ring.Accent = color; _ring.Invalidate();
        var previous = _tray.Icon;
        _tray.Icon = CreateTrayIcon(color);
        previous?.Dispose();
    }

    private Drawing.Icon CreateTrayIcon(Drawing.Color color)
    {
        using var bitmap = new Drawing.Bitmap(32, 32);
        using (var graphics = Drawing.Graphics.FromImage(bitmap))
        {
            graphics.Clear(Drawing.Color.Transparent);
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            using var outline = new Drawing.Pen(Drawing.Color.FromArgb(18, 38, 58), 2);
            using var fill = new Drawing.SolidBrush(color);
            graphics.FillRectangle(fill, 5, 10, 21, 16);
            graphics.DrawRectangle(outline, 5, 10, 21, 16);
            graphics.FillRectangle(fill, 11, 6, 9, 4);
            graphics.DrawRectangle(outline, 11, 6, 9, 4);
            using var fin = new Drawing.SolidBrush(Drawing.Color.FromArgb(34, 148, 224));
            graphics.FillPolygon(fin, new Drawing.Point[] { new(8, 21), new(14, 11), new(18, 21) });
        }
        var handle = bitmap.GetHicon();
        try
        {
            using var icon = Drawing.Icon.FromHandle(handle);
            return (Drawing.Icon)icon.Clone();
        }
        finally { DestroyIcon(handle); }
    }

    private Drawing.Icon CreateAppIcon()
    {
        using var bitmap = new Drawing.Bitmap(32, 32);
        using (var graphics = Drawing.Graphics.FromImage(bitmap))
        {
            graphics.Clear(Drawing.Color.Transparent);
            graphics.DrawImage(_mascot, new Drawing.Rectangle(0, 0, 32, 32));
        }
        var handle = bitmap.GetHicon();
        try
        {
            using var icon = Drawing.Icon.FromHandle(handle);
            return (Drawing.Icon)icon.Clone();
        }
        finally { DestroyIcon(handle); }
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool DestroyIcon(IntPtr hIcon);
    private void ShowPanel() { Show(); WindowState = Forms.FormWindowState.Normal; Activate(); }
    private void ExitProgram() { _exitRequested = true; _tray.Dispose(); Close(); }
    protected override void Dispose(bool disposing) { if (disposing) { _tray.Dispose(); _mascot.Dispose(); _taskbarMascot.Dispose(); } base.Dispose(disposing); }
}

internal sealed record ProbeReading(int? Percent, bool Charging, string Message);

internal static class ProbeRunner
{
    public static async Task<ProbeReading> ReadAsync()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "r5-battery-probe.exe");
        if (!File.Exists(path)) return new(null, false, "Falta el lector HID de la aplicación.");
        try
        {
            using var process = Process.Start(new ProcessStartInfo(path, "--probe-once") { RedirectStandardOutput = true, UseShellExecute = false, CreateNoWindow = true });
            if (process is null) return new(null, false, "No se pudo iniciar el lector HID.");
            var output = await process.StandardOutput.ReadToEndAsync();
            await process.WaitForExitAsync();
            using var json = JsonDocument.Parse(output);
            if (json.RootElement.GetProperty("status").GetString() != "ok") return new(null, false, "El mouse respondió con un formato aún no validado.");
            var reading = json.RootElement.GetProperty("reading");
            return new(reading.GetProperty("percent").GetInt32(), reading.GetProperty("charging").GetBoolean(), "");
        }
        catch { return new(null, false, "No se pudo leer el receptor R5 Ultra."); }
    }
}
