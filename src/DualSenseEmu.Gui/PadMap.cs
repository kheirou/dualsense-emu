using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;
using Shape = Avalonia.Controls.Shapes.Path;

namespace DualSenseEmu.Gui;

/// <summary>
/// A vector gamepad (DualSense or Xbox 360) drawn on a 260x170 canvas: flat body, hairline outline,
/// muted idle buttons, pressed buttons fill with one accent. Only the face buttons keep their real colours.
/// </summary>
public sealed class PadMap
{
    static IBrush B(string hex) => new SolidColorBrush(Color.Parse(hex));
    const string Accent = "#4FB7D6", Edge = "#3A4256", Ring = "#7A8296";
    static readonly IBrush Body = B("#1A1E28"), Off = B("#11141C"), LabelOff = B("#A3A9B8");
    static readonly IBrush LabelOn = B("#0B0D14"); // dark text on the accent pressed fill

    readonly Dictionary<string, (Border Box, TextBlock? Label, IBrush LabelOff, IBrush On)> _parts = new();
    Shape _l2 = null!, _r2 = null!;
    Ellipse _leftDot = null!, _rightDot = null!;
    Point _leftC, _rightC;
    readonly Canvas _canvas;

    public PadMap(Canvas canvas, bool xbox)
    {
        _canvas = canvas;
        canvas.Width = 260; canvas.Height = 170;

        Add(new Shape
        {
            Data = Geometry.Parse("M 40,24 C 40,18 55,18 75,18 L 185,18 C 205,18 220,18 220,24 C 236,70 252,128 234,150 C 220,166 196,162 186,142 L 176,122 L 84,122 L 74,142 C 64,162 40,166 26,150 C 8,128 24,70 40,24 Z"),
            Fill = Body, Stroke = B(Edge), StrokeThickness = 1,
        });
        // shoulders: curved trigger caps on the outer corners (fill with pull), bumper keys just inside them
        _l2 = Trigger("M 30,27 C 30,12 40,4 54,4 L 60,4 C 63,4 64,6 64,9 L 64,16 C 52,16 42,20 37,28 Z", 41, 6, xbox ? "LT" : "L2");
        _r2 = Trigger("M 230,27 C 230,12 220,4 206,4 L 200,4 C 197,4 196,6 196,9 L 196,16 C 208,16 218,20 223,28 Z", 199, 6, xbox ? "RT" : "R2");
        Part(xbox ? "LeftShoulder" : "L1", 68, 7, 24, 12, 4, xbox ? "LB" : "L1", 7);
        Part(xbox ? "RightShoulder" : "R1", 168, 7, 24, 12, 4, xbox ? "RB" : "R1", 7);

        if (xbox)
        {
            _leftC = new Point(62, 56); _rightC = new Point(162, 104);
            Dpad(98, 104);
            Part("Back", 106, 52, 14, 9, 4, "◀", 5);
            Part("Start", 140, 52, 14, 9, 4, "▶", 5);
            Part("Guide", 120, 29, 20, 20, 10, "✕", 10);
            Part("Y", 189, 42, 18, 18, 9, "Y", 10, colour: "#E0A23F");
            Part("B", 207, 60, 18, 18, 9, "B", 10, colour: "#E5484D");
            Part("A", 189, 78, 18, 18, 9, "A", 10, colour: "#4CC38A");
            Part("X", 171, 60, 18, 18, 9, "X", 10, colour: "#4FB7D6");
        }
        else
        {
            _leftC = new Point(98, 104); _rightC = new Point(162, 104);
            Dpad(62, 66);
            Part("Touchpad", 100, 28, 60, 26, 6, null, 0);
            Part("Create", 84, 32, 8, 12, 4, null, 0);
            Part("Options", 168, 32, 8, 12, 4, null, 0);
            Part("Ps", 123, 66, 14, 14, 7, "PS", 5);
            Part("Mute", 124, 86, 12, 5, 2, null, 0);
            Part("Triangle", 189, 42, 18, 18, 9, "△", 10, colour: "#4CC38A");
            Part("Circle", 207, 60, 18, 18, 9, "○", 10, colour: "#E5484D");
            Part("Cross", 189, 78, 18, 18, 9, "✕", 10, colour: "#5B8DEF");
            Part("Square", 171, 60, 18, 18, 9, "□", 10, colour: "#C97BD6");
        }
        // sticks: hairline rings with crosshair lines (fill on L3/R3); the accent dot follows the stick
        Well(xbox ? "LeftThumb" : "L3", _leftC);
        Well(xbox ? "RightThumb" : "R3", _rightC);
        _leftDot = Dot(_leftC); _rightDot = Dot(_rightC);
    }

    void Add(Control c, double x = 0, double y = 0)
    {
        Canvas.SetLeft(c, x); Canvas.SetTop(c, y);
        _canvas.Children.Add(c);
    }

    void Dpad(double cx, double cy)
    {
        Part("DpadUp", cx - 7, cy - 21, 14, 14, 3, "▲", 7);
        Part("DpadDown", cx - 7, cy + 7, 14, 14, 3, "▼", 7);
        Part("DpadLeft", cx - 21, cy - 7, 14, 14, 3, "◀", 7);
        Part("DpadRight", cx + 7, cy - 7, 14, 14, 3, "▶", 7);
    }

    void Well(string key, Point c)
    {
        Part(key, c.X - 19, c.Y - 19, 38, 38, 19, null, 0, edgeWidth: 1.5);
        var line = B("#55" + Ring.TrimStart('#'));
        Add(new Ellipse { Width = 24, Height = 24, Stroke = line, StrokeThickness = 0.6, IsHitTestVisible = false }, c.X - 12, c.Y - 12);
        Add(new Line { StartPoint = new Point(c.X - 16, c.Y), EndPoint = new Point(c.X + 16, c.Y), Stroke = line, StrokeThickness = 0.5, IsHitTestVisible = false });
        Add(new Line { StartPoint = new Point(c.X, c.Y - 16), EndPoint = new Point(c.X, c.Y + 16), Stroke = line, StrokeThickness = 0.5, IsHitTestVisible = false });
    }

    /// <summary>An outlined shoulder cap plus an accent copy whose opacity follows the trigger pull.</summary>
    Shape Trigger(string geometry, double labelX, double labelY, string label)
    {
        Add(new Shape { Data = Geometry.Parse(geometry), Fill = Off, Stroke = B(Edge), StrokeThickness = 1 });
        var fill = new Shape { Data = Geometry.Parse(geometry), Fill = B(Accent), Opacity = 0 };
        Add(fill);
        Add(new TextBlock { Text = label, FontSize = 6, FontWeight = FontWeight.Medium, Foreground = LabelOff, IsHitTestVisible = false }, labelX, labelY);
        return fill;
    }

    /// <param name="colour">Own colour for outline, label and pressed fill (face buttons); null uses the neutral look and the accent.</param>
    void Part(string key, double x, double y, double w, double h, double r, string? label, double font,
              string? colour = null, double edgeWidth = 1)
    {
        var labelOff = colour is null ? LabelOff : B(colour);
        TextBlock? tb = null;
        var box = new Border
        {
            Width = w, Height = h, CornerRadius = new CornerRadius(r),
            Background = Off, BorderBrush = B(colour ?? Edge), BorderThickness = new Thickness(edgeWidth),
        };
        if (label is not null)
        {
            tb = new TextBlock
            {
                Text = label, FontSize = font, FontWeight = FontWeight.Medium, Foreground = labelOff,
                HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center,
            };
            box.Child = tb;
        }
        Add(box, x, y);
        _parts[key] = (box, tb, labelOff, B(colour ?? Accent));
    }

    Ellipse Dot(Point c)
    {
        var d = new Ellipse { Width = 8, Height = 8, Fill = B(Accent), IsHitTestVisible = false };
        Add(d, c.X - 4.5, c.Y - 4.5);
        return d;
    }

    /// <summary>Sticks are raw 0-255 (centre 128); triggers 0-255.</summary>
    public void Update(Func<string, bool> pressed, byte l2, byte r2, byte lx, byte ly, byte rx, byte ry)
    {
        foreach (var (key, (box, label, labelOff, on)) in _parts)
        {
            bool p = pressed(key);
            box.Background = p ? on : Off;
            if (label is not null) label.Foreground = p ? LabelOn : labelOff;
        }
        _l2.Opacity = l2 / 255.0; _r2.Opacity = r2 / 255.0;
        Move(_leftDot, _leftC, lx, ly); Move(_rightDot, _rightC, rx, ry);
    }

    static void Move(Ellipse dot, Point c, byte x, byte y)
    {
        // +-12px of travel inside the 38px well
        Canvas.SetLeft(dot, c.X - 4 + (x - 128) / 128.0 * 12);
        Canvas.SetTop(dot, c.Y - 4 + (y - 128) / 128.0 * 12);
    }
}
