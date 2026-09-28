using Drawing = System.Drawing;
using Forms = System.Windows.Forms;
using System.Drawing.Drawing2D;

namespace R5BatteryEstimator.Native;

internal sealed class BatteryChart : Forms.Control
{
    private IReadOnlyList<BatteryObservation> _items = [];

    public IReadOnlyList<BatteryObservation> Items { get => _items; set { _items = value; Invalidate(); } }

    public BatteryChart()
    {
        DoubleBuffered = true;
        BackColor = Drawing.Color.FromArgb(16, 27, 35);
        ForeColor = Drawing.Color.FromArgb(205, 220, 243);
    }

    protected override void OnPaint(Forms.PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        using var card = new GraphicsPath(); card.AddArc(0, 0, 18, 18, 180, 90); card.AddArc(Width - 19, 0, 18, 18, 270, 90); card.AddArc(Width - 19, Height - 19, 18, 18, 0, 90); card.AddArc(0, Height - 19, 18, 18, 90, 90); card.CloseFigure();
        using var fill = new Drawing.SolidBrush(BackColor); using var border = new Drawing.Pen(Drawing.Color.FromArgb(35, 52, 64)); e.Graphics.FillPath(fill, card); e.Graphics.DrawPath(border, card);
        using var heading = new Drawing.Font("Segoe UI", 14, Drawing.FontStyle.Bold); e.Graphics.DrawString("▮▮   HISTORIAL: HORA / PORCENTAJE", heading, new Drawing.SolidBrush(ForeColor), 42, 31);
        var bounds = new Drawing.Rectangle(106, 95, Math.Max(1, Width - 140), Math.Max(1, Height - 145));
        using var grid = new Drawing.Pen(Drawing.Color.FromArgb(51, 73, 86)) { DashStyle = DashStyle.Dash };
        for (var line = 0; line <= 4; line++)
        {
            var y = bounds.Top + bounds.Height * line / 4;
            e.Graphics.DrawLine(grid, bounds.Left, y, bounds.Right, y);
        }
        for (var line = 1; line < 4; line++) { var x = bounds.Left + bounds.Width * line / 4; e.Graphics.DrawLine(grid, x, bounds.Top, x, bounds.Bottom); }
        using var labelFont = new Drawing.Font("Segoe UI", 11);
        e.Graphics.DrawString("100%", labelFont, new Drawing.SolidBrush(ForeColor), 30, bounds.Top - 8);
        e.Graphics.DrawString("0%", labelFont, new Drawing.SolidBrush(ForeColor), 54, bounds.Bottom - 8);
        e.Graphics.DrawString("24 h", labelFont, new Drawing.SolidBrush(ForeColor), bounds.Left, bounds.Bottom + 18);
        e.Graphics.DrawString("ahora", labelFont, new Drawing.SolidBrush(ForeColor), bounds.Right - 45, bounds.Bottom + 18);
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
        using var area = new GraphicsPath(); area.AddLines(points); area.AddLine(points[^1].X, points[^1].Y, bounds.Right, bounds.Bottom); area.AddLine(bounds.Right, bounds.Bottom, bounds.Left, bounds.Bottom); area.CloseFigure();
        using var areaBrush = new LinearGradientBrush(bounds, Drawing.Color.FromArgb(85, 26, 226, 133), Drawing.Color.FromArgb(7, 26, 226, 133), LinearGradientMode.Vertical); e.Graphics.FillPath(areaBrush, area);
        using var linePen = new Drawing.Pen(Drawing.Color.FromArgb(43, 233, 138), 3) { StartCap = LineCap.Round, EndCap = LineCap.Round, LineJoin = LineJoin.Round };
        e.Graphics.DrawLines(linePen, points);
    }
}
