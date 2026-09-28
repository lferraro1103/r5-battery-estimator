using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class StatusRing : Forms.Control
{
    public int Percent { get; set; }
    public string Status { get; set; } = "No cargando";
    public Drawing.Color Accent { get; set; } = Drawing.Color.FromArgb(47, 225, 137);
    public StatusRing() { DoubleBuffered = true; Size = new Drawing.Size(370, 370); BackColor = Drawing.Color.Transparent; }
    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = Drawing.Drawing2D.SmoothingMode.AntiAlias;
        var ring = new Drawing.Rectangle(24, 24, Width - 48, Height - 48);
        using var basePen = new Drawing.Pen(Drawing.Color.FromArgb(40, 51, 62), 22);
        using var valuePen = new Drawing.Pen(Accent, 22) { StartCap = Drawing.Drawing2D.LineCap.Round, EndCap = Drawing.Drawing2D.LineCap.Round };
        e.Graphics.DrawArc(basePen, ring, -90, 360);
        e.Graphics.DrawArc(valuePen, ring, -90, Percent * 3.6f);
        using var valueFont = new Drawing.Font("Segoe UI", 60, Drawing.FontStyle.Bold);
        using var statusFont = new Drawing.Font("Segoe UI", 16);
        var value = $"{Percent}%"; var valueSize = e.Graphics.MeasureString(value, valueFont);
        e.Graphics.DrawString(value, valueFont, Drawing.Brushes.White, (Width - valueSize.Width) / 2, 125);
        using var batteryPen = new Drawing.Pen(Accent, 4);
        e.Graphics.DrawRectangle(batteryPen, Width / 2 - 22, 215, 40, 20); e.Graphics.DrawLine(batteryPen, Width / 2 + 20, 221, Width / 2 + 26, 221);
        var statusSize = e.Graphics.MeasureString(Status, statusFont);
        e.Graphics.DrawString(Status, statusFont, Drawing.Brushes.LightSteelBlue, (Width - statusSize.Width) / 2, 252);
    }
}
