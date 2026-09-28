using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class StatusRing : Forms.Control
{
    public int Percent { get; set; }
    public string Status { get; set; } = "No cargando";
    public Drawing.Color Accent { get; set; } = Drawing.Color.FromArgb(47, 225, 137);
    public StatusRing()
    {
        SetStyle(Forms.ControlStyles.SupportsTransparentBackColor, true);
        DoubleBuffered = true;
        Size = new Drawing.Size(370, 370);
        BackColor = Drawing.Color.Transparent;
    }
    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = Drawing.Drawing2D.SmoothingMode.AntiAlias;
        var size = Math.Min(Width, Height); var inset = Math.Max(34, size / 9); var ring = new Drawing.Rectangle(inset, inset, size - inset * 2, size - inset * 2);
        using var glowPen = new Drawing.Pen(Drawing.Color.FromArgb(24, Accent), Math.Max(36, size / 7)) { StartCap = Drawing.Drawing2D.LineCap.Round, EndCap = Drawing.Drawing2D.LineCap.Round };
        e.Graphics.DrawArc(glowPen, ring, -90, Percent * 3.6f);
        using var basePen = new Drawing.Pen(Drawing.Color.FromArgb(40, 51, 62), 22);
        using var valuePen = new Drawing.Pen(Accent, 22) { StartCap = Drawing.Drawing2D.LineCap.Round, EndCap = Drawing.Drawing2D.LineCap.Round };
        e.Graphics.DrawArc(basePen, ring, -90, 360);
        e.Graphics.DrawArc(valuePen, ring, -90, Percent * 3.6f);
        using var valueFont = new Drawing.Font("Segoe UI", size / 6.2f, Drawing.FontStyle.Bold);
        using var statusFont = new Drawing.Font("Segoe UI", size / 25f);
        var value = $"{Percent}%"; var valueSize = e.Graphics.MeasureString(value, valueFont);
        e.Graphics.DrawString(value, valueFont, Drawing.Brushes.White, (size - valueSize.Width) / 2, size * .32f);
        using var batteryPen = new Drawing.Pen(Accent, 4);
        var batteryWidth = size / 9; var batteryHeight = size / 18; var batteryLeft = size / 2 - (batteryWidth + 6) / 2; e.Graphics.DrawRectangle(batteryPen, batteryLeft, (int)(size * .56f), batteryWidth, batteryHeight); e.Graphics.DrawLine(batteryPen, batteryLeft + batteryWidth, (int)(size * .58f), batteryLeft + batteryWidth + 6, (int)(size * .58f));
        var statusSize = e.Graphics.MeasureString(Status, statusFont);
        e.Graphics.DrawString(Status, statusFont, Drawing.Brushes.LightSteelBlue, (size - statusSize.Width) / 2, size * .65f);
    }
}
