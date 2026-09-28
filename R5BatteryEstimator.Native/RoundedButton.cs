using System.Drawing.Drawing2D;
using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class RoundedButton : Forms.Button
{
    public RoundedButton()
    {
        FlatAppearance.BorderSize = 0;
        FlatAppearance.MouseOverBackColor = Drawing.Color.FromArgb(245, 69, 75);
        FlatAppearance.MouseDownBackColor = Drawing.Color.FromArgb(196, 39, 45);
    }

    protected override void OnResize(EventArgs e)
    {
        base.OnResize(e);
        using var path = new GraphicsPath(); const int radius = 12;
        path.AddArc(0, 0, radius, radius, 180, 90); path.AddArc(Width - radius - 1, 0, radius, radius, 270, 90);
        path.AddArc(Width - radius - 1, Height - radius - 1, radius, radius, 0, 90); path.AddArc(0, Height - radius - 1, radius, radius, 90, 90); path.CloseFigure(); Region = new Drawing.Region(path);
    }
}
