using Drawing = System.Drawing;
using Forms = System.Windows.Forms;

namespace R5BatteryEstimator.Native;

internal sealed class BatteryChart : Forms.Control
{
    private IReadOnlyList<BatteryObservation> _items = [];

    public IReadOnlyList<BatteryObservation> Items { get => _items; set { _items = value; Invalidate(); } }

    public BatteryChart()
    {
        DoubleBuffered = true;
        BackColor = Drawing.Color.FromArgb(43, 45, 49);
        ForeColor = Drawing.Color.FromArgb(185, 190, 200);
    }

    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        base.OnPaint(e);
        var bounds = new Drawing.Rectangle(40, 18, Math.Max(1, Width - 54), Math.Max(1, Height - 42));
        using var grid = new Drawing.Pen(Drawing.Color.FromArgb(70, 76, 81));
        for (var line = 0; line <= 4; line++)
        {
            var y = bounds.Top + bounds.Height * line / 4;
            e.Graphics.DrawLine(grid, bounds.Left, y, bounds.Right, y);
        }
        using var labelFont = new Drawing.Font("Segoe UI", 8);
        e.Graphics.DrawString("100%", labelFont, new Drawing.SolidBrush(ForeColor), 2, bounds.Top - 6);
        e.Graphics.DrawString("0%", labelFont, new Drawing.SolidBrush(ForeColor), 12, bounds.Bottom - 8);
        e.Graphics.DrawString("24 h", labelFont, new Drawing.SolidBrush(ForeColor), bounds.Left, bounds.Bottom + 6);
        e.Graphics.DrawString("ahora", labelFont, new Drawing.SolidBrush(ForeColor), bounds.Right - 31, bounds.Bottom + 6);
        if (_items.Count < 2)
        {
            var text = "El gráfico aparece al reunir lecturas válidas.";
            var size = e.Graphics.MeasureString(text, labelFont);
            e.Graphics.DrawString(text, labelFont, new Drawing.SolidBrush(ForeColor), bounds.Left + (bounds.Width - size.Width) / 2, bounds.Top + bounds.Height / 2 - size.Height / 2);
            return;
        }
        var start = DateTimeOffset.Now.AddHours(-24);
        var points = _items.Select(item => new Drawing.PointF(
            bounds.Left + (float)Math.Clamp((item.At - start).TotalHours / 24, 0, 1) * bounds.Width,
            bounds.Bottom - item.Percent / 100f * bounds.Height)).ToArray();
        using var linePen = new Drawing.Pen(Drawing.Color.FromArgb(47, 204, 113), 2);
        e.Graphics.DrawLines(linePen, points);
    }
}
