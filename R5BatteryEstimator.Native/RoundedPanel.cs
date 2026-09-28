using System.Drawing.Drawing2D;
using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class RoundedPanel : Forms.Panel
{
    public RoundedPanel() => DoubleBuffered = true;

    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        using var path = Path();
        using var fill = new Drawing.SolidBrush(BackColor);
        using var border = new Drawing.Pen(Drawing.Color.FromArgb(35, 52, 64));
        e.Graphics.FillPath(fill, path);
        e.Graphics.DrawPath(border, path);
    }

    protected override void OnResize(EventArgs e) { base.OnResize(e); using var path = Path(); Region = new Drawing.Region(path); }

    private GraphicsPath Path()
    {
        var p = new GraphicsPath(); var r = new Drawing.Rectangle(0, 0, Math.Max(1, Width - 1), Math.Max(1, Height - 1)); const int radius = 16;
        p.AddArc(r.Left, r.Top, radius, radius, 180, 90); p.AddArc(r.Right - radius, r.Top, radius, radius, 270, 90);
        p.AddArc(r.Right - radius, r.Bottom - radius, radius, radius, 0, 90); p.AddArc(r.Left, r.Bottom - radius, radius, radius, 90, 90); p.CloseFigure(); return p;
    }
}
