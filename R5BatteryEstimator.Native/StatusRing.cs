using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class StatusRing : Forms.Control
{
    public int Percent { get; set; }
    public string Status { get; set; } = "No cargando";
    public Drawing.Color Accent { get; set; } = Drawing.Color.FromArgb(47, 225, 137);
    // WinForms controls cannot use a transparent background unless their parent
    // explicitly supports it. Match the dashboard surface instead so startup
    // remains reliable on every Windows configuration.
    public StatusRing() { DoubleBuffered = true; Size = new Drawing.Size(370, 370); BackColor = Drawing.Color.FromArgb(32, 33, 36); }
    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = Drawing.Drawing2D.SmoothingMode.AntiAlias;
        var size = Math.Min(Width, Height); var inset = Math.Max(20, size / 15); var ring = new Drawing.Rectangle(inset, inset, size - inset * 2, size - inset * 2);
        using var glowPen = new Drawing.Pen(Drawing.Color.FromArgb(32, Accent), Math.Max(42, size / 6));
        e.Graphics.DrawArc(glowPen, ring, -90, Percent * 3.6f);
        using var basePen = new Drawing.Pen(Drawing.Color.FromArgb(40, 51, 62), 22);
        using var valuePen = new Drawing.Pen(Accent, 22) { StartCap = Drawing.Drawing2D.LineCap.Round, EndCap = Drawing.Drawing2D.LineCap.Round };
        e.Graphics.DrawArc(basePen, ring, -90, 360);
        e.Graphics.DrawArc(valuePen, ring, -90, Percent * 3.6f);
        using var valueFont = new Drawing.Font("Segoe UI", size / 5.5f, Drawing.FontStyle.Bold);
        using var statusFont = new Drawing.Font("Segoe UI", size / 22f);
        var value = $"{Percent}%"; var valueSize = e.Graphics.MeasureString(value, valueFont);
        e.Graphics.DrawString(value, valueFont, Drawing.Brushes.White, (size - valueSize.Width) / 2, size * .34f);
        using var batteryPen = new Drawing.Pen(Accent, 4);
        var batteryWidth = size / 8; var batteryHeight = size / 16; e.Graphics.DrawRectangle(batteryPen, size / 2 - batteryWidth / 2, (int)(size * .63f), batteryWidth, batteryHeight); e.Graphics.DrawLine(batteryPen, size / 2 + batteryWidth / 2, (int)(size * .65f), size / 2 + batteryWidth / 2 + 6, (int)(size * .65f));
        var statusSize = e.Graphics.MeasureString(Status, statusFont);
        e.Graphics.DrawString(Status, statusFont, Drawing.Brushes.LightSteelBlue, (size - statusSize.Width) / 2, size * .76f);
    }
}
