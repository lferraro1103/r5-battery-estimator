using System.Windows;
using Forms = System.Windows.Forms;
using Drawing = System.Drawing;

namespace R5BatteryEstimator.Native;

public partial class App : System.Windows.Application
{
    private Forms.NotifyIcon? _trayIcon;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add("Abrir panel", null, (_, _) => ShowPanel());
        menu.Items.Add("Actualizar perfil", null, (_, _) => CurrentWindow?.MarkProfileChecked());
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add("Cerrar programa", null, (_, _) => ExitProgram());

        _trayIcon = new Forms.NotifyIcon
        {
            Icon = Drawing.SystemIcons.Application,
            Text = "R5 Battery Estimator",
            ContextMenuStrip = menu,
            Visible = true,
        };
        _trayIcon.MouseUp += (_, args) =>
        {
            if (args.Button == Forms.MouseButtons.Left)
                menu.Show(Forms.Cursor.Position);
        };
    }

    internal MainWindow? CurrentWindow => MainWindow as MainWindow;

    internal void HideToTray() => CurrentWindow?.Hide();

    private void ShowPanel()
    {
        var window = CurrentWindow;
        if (window is null) return;
        window.Show();
        window.WindowState = WindowState.Normal;
        window.Activate();
    }

    private void ExitProgram()
    {
        _trayIcon?.Dispose();
        Shutdown();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _trayIcon?.Dispose();
        base.OnExit(e);
    }
}
