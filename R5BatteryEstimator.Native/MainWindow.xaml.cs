using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Threading;

namespace R5BatteryEstimator.Native;

public partial class MainWindow : Window
{
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromSeconds(30) };

    public MainWindow()
    {
        InitializeComponent();
        Loaded += async (_, _) => await RefreshAsync();
        _timer.Tick += async (_, _) => await RefreshAsync();
        _timer.Start();
    }

    private async void Refresh_Click(object sender, RoutedEventArgs e) => await RefreshAsync();

    protected override void OnStateChanged(EventArgs e)
    {
        base.OnStateChanged(e);
        if (WindowState == WindowState.Minimized) ((App)System.Windows.Application.Current).HideToTray();
    }

    protected override void OnClosing(System.ComponentModel.CancelEventArgs e)
    {
        e.Cancel = true;
        ((App)System.Windows.Application.Current).HideToTray();
    }

    internal void MarkProfileChecked()
    {
        StatusText.Text = "Perfil 1 revisado. Se actualizará al detectar cambios del mouse.";
        UpdatedText.Text = DateTime.Now.ToString("HH:mm");
    }

    private async Task RefreshAsync()
    {
        StatusText.Text = "Leyendo el receptor…";
        var result = await ProbeRunner.ReadAsync();
        if (result.Percent is not int percent)
        {
            PercentText.Text = "—%";
            BatteryBar.Value = 0;
            EstimateText.Text = "—";
            UpdatedText.Text = "Sin datos";
            StatusText.Text = result.Message;
            return;
        }

        PercentText.Text = $"{percent}%";
        BatteryBar.Value = percent;
        EstimateText.Text = $"{Math.Round(percent * 2.0):0} h";
        UpdatedText.Text = DateTime.Now.ToString("HH:mm");
        StatusText.Text = result.Charging ? "Cargando" : "No cargando";
    }
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
            using var process = Process.Start(new ProcessStartInfo(path, "--probe-once") { RedirectStandardOutput = true, RedirectStandardError = true, UseShellExecute = false, CreateNoWindow = true });
            if (process is null) return new(null, false, "No se pudo iniciar el lector HID.");
            var output = await process.StandardOutput.ReadToEndAsync();
            await process.WaitForExitAsync();
            using var json = JsonDocument.Parse(output);
            var root = json.RootElement;
            if (root.GetProperty("status").GetString() != "ok") return new(null, false, "El mouse respondió con un formato aún no validado.");
            var reading = root.GetProperty("reading");
            return new(reading.GetProperty("percent").GetInt32(), reading.GetProperty("charging").GetBoolean(), "");
        }
        catch (Exception)
        {
            return new(null, false, "No se pudo leer el receptor R5 Ultra.");
        }
    }
}
