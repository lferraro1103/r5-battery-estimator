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
    private readonly Forms.Label _updated = new() { Text = "Sin datos", Font = new Drawing.Font("Segoe UI", 16, Drawing.FontStyle.Bold), AutoSize = true, ForeColor = Drawing.Color.White };
    private readonly Forms.Panel _circle = new() { Size = new Drawing.Size(142, 142), BackColor = Drawing.Color.DimGray };
    private readonly BatteryHistory _history = new();
    private readonly BatteryChart _chart = new() { Location = new Drawing.Point(30, 377), Size = new Drawing.Size(600, 145) };
    private readonly Forms.Timer _timer = new() { Interval = 30_000 };
    private readonly Forms.NotifyIcon _tray;
    private readonly Drawing.Image _mascot;
    private bool _exitRequested;

    public BatteryForm()
    {
        Text = "R5 Battery Estimator";
        ClientSize = new Drawing.Size(660, 590);
        FormBorderStyle = Forms.FormBorderStyle.FixedSingle;
        MaximizeBox = false;
        BackColor = Drawing.Color.FromArgb(32, 33, 36);
        ForeColor = Drawing.Color.White;
        StartPosition = Forms.FormStartPosition.CenterScreen;

        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add("Abrir panel", null, (_, _) => ShowPanel());
        menu.Items.Add("Actualizar perfil", null, (_, _) => MarkProfileChecked());
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add("Cerrar programa", null, (_, _) => ExitProgram());
        _mascot = Drawing.Image.FromFile(Path.Combine(AppContext.BaseDirectory, "Assets", "shark-battery.png"));
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
        var title = new Forms.Label { Text = "R5 Battery Estimator", Font = new Drawing.Font("Segoe UI", 19, Drawing.FontStyle.Bold), AutoSize = true, Location = new Drawing.Point(28, 22), ForeColor = Drawing.Color.White };
        var badge = new Forms.Label { Text = "R5 ULTRA", AutoSize = true, Location = new Drawing.Point(475, 28), ForeColor = Drawing.Color.FromArgb(230, 80, 75), Font = new Drawing.Font("Segoe UI", 9, Drawing.FontStyle.Bold) };
        var panelLogo = new Forms.PictureBox { Image = _mascot, Location = new Drawing.Point(570, 10), Size = new Drawing.Size(58, 58), SizeMode = Forms.PictureBoxSizeMode.Zoom };
        var caption = new Forms.Label { Text = "BATERÍA ACTUAL", AutoSize = true, Location = new Drawing.Point(30, 79), ForeColor = Drawing.Color.FromArgb(185, 190, 200), Font = new Drawing.Font("Segoe UI", 9, Drawing.FontStyle.Bold) };
        _circle.Location = new Drawing.Point(30, 98);
        using (var path = new GraphicsPath()) { path.AddEllipse(0, 0, _circle.Width, _circle.Height); _circle.Region = new Drawing.Region(path); }
        _circle.Controls.Add(_percent);
        _status.Location = new Drawing.Point(195, 158);
        var left = Card("DURACIÓN DE CARGA COMPLETA", _estimate, "Se aprende con tu descarga real.", 30);
        var right = Card("ÚLTIMA LECTURA", _updated, "Actualiza cada 30 segundos.", 335);
        left.Top = 267; right.Top = 267;
        var chartTitle = new Forms.Label { Text = "HISTORIAL: HORA / PORCENTAJE", AutoSize = true, Location = new Drawing.Point(30, 356), ForeColor = Drawing.Color.FromArgb(185, 190, 200), Font = new Drawing.Font("Segoe UI", 9, Drawing.FontStyle.Bold) };
        var hint = new Forms.Label { Text = "La app nunca muestra una desconexión como 0%.", AutoSize = true, Location = new Drawing.Point(30, 557), ForeColor = Drawing.Color.FromArgb(185, 190, 200) };
        var update = new Forms.Button { Text = "Actualizar ahora", AutoSize = true, Location = new Drawing.Point(500, 547), BackColor = Drawing.Color.FromArgb(220, 55, 55), ForeColor = Drawing.Color.White, FlatStyle = Forms.FlatStyle.Flat };
        update.FlatAppearance.BorderSize = 0; update.Click += async (_, _) => await RefreshAsync();
        Controls.AddRange([title, badge, panelLogo, caption, _circle, _status, left, right, chartTitle, _chart, hint, update]);
    }

    private Forms.Panel Card(string title, Forms.Label value, string note, int left)
    {
        var card = new Forms.Panel { Location = new Drawing.Point(left, 250), Size = new Drawing.Size(295, 92), BackColor = Drawing.Color.FromArgb(43, 45, 49) };
        card.Controls.Add(new Forms.Label { Text = title, AutoSize = true, Location = new Drawing.Point(14, 12), ForeColor = Drawing.Color.FromArgb(185, 190, 200), Font = new Drawing.Font("Segoe UI", 8, Drawing.FontStyle.Bold) });
        value.Location = new Drawing.Point(14, 30); card.Controls.Add(value);
        card.Controls.Add(new Forms.Label { Text = note, AutoSize = true, Location = new Drawing.Point(14, 67), ForeColor = Drawing.Color.FromArgb(185, 190, 200), Font = new Drawing.Font("Segoe UI", 8) });
        return card;
    }

    private async Task RefreshAsync()
    {
        _status.Text = "Leyendo el receptor…";
        var result = await ProbeRunner.ReadAsync();
        if (result.Percent is not int percent) { _percent.Text = "—%"; _estimate.Text = _history.LearnedFullChargeHours() is double previous ? $"{previous:0} h" : "Aprendiendo"; _updated.Text = "Sin datos"; _status.Text = result.Message; SetTrayText("R5 Battery Estimator — sin lectura válida"); SetTrayColor(Drawing.Color.DimGray); return; }
        var hours = Math.Round(percent * 2.0);
        _history.Add(percent, result.Charging);
        _chart.Items = _history.Recent(TimeSpan.FromHours(24));
        _percent.Text = $"{percent}%"; _estimate.Text = _history.LearnedFullChargeHours() is double learned ? $"{learned:0} h" : "Aprendiendo"; _updated.Text = DateTime.Now.ToString("HH:mm"); _status.Text = result.Charging ? "Cargando" : "No cargando";
        SetTrayColor(percent > 50 ? Drawing.Color.FromArgb(46, 204, 113) : percent > 20 ? Drawing.Color.FromArgb(241, 196, 15) : Drawing.Color.FromArgb(231, 76, 60));
        SetTrayText($"R5: {percent}% — hasta {hours:0} h (provisional)");
    }

    private void MarkProfileChecked() { _updated.Text = DateTime.Now.ToString("HH:mm"); _status.Text = "Perfil 1 revisado. Se actualizará al detectar cambios del mouse."; }
    private void SetTrayText(string text) => _tray.Text = text.Length <= 63 ? text : text[..63];
    private void SetTrayColor(Drawing.Color color)
    {
        _circle.BackColor = color;
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
            using var pen = new Drawing.Pen(color, 3);
            graphics.DrawEllipse(pen, 1, 1, 29, 29);
            graphics.DrawImage(_mascot, new Drawing.Rectangle(2, 2, 28, 28), 145, 110, 735, 850, Drawing.GraphicsUnit.Pixel);
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
            graphics.DrawImage(_mascot, new Drawing.Rectangle(0, 0, 32, 32), 145, 110, 735, 850, Drawing.GraphicsUnit.Pixel);
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
    protected override void Dispose(bool disposing) { if (disposing) { _tray.Dispose(); _mascot.Dispose(); } base.Dispose(disposing); }
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
