using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;
using Shape = Avalonia.Controls.Shapes.Path;

namespace DualSenseEmu.Gui;

/// <summary>
/// A vector gamepad (DualSense or Xbox 360) drawn on a 260x170 canvas in a neon "glass" style:
/// translucent body, glowing outline, pressed buttons fill with the pad's accent colour.
/// </summary>
public sealed class PadMap
{
    static IBrush B(string hex) => new SolidColorBrush(Color.Parse(hex));
    static IBrush Vertical(string top, string bottom) => new LinearGradientBrush
    {
        StartPoint = new RelativePoint(0.5, 0, RelativeUnit.Relative),
        EndPoint = new RelativePoint(0.5, 1, RelativeUnit.Relative),
        GradientStops = { new GradientStop(Color.Parse(top), 0), new GradientStop(Color.Parse(bottom), 1) },
    };

    // one look per pad: glass body, glowing edge, idle button fill, idle label, accent, stick ring colours
    sealed record Look(IBrush Body, string Edge, IBrush Off, IBrush LabelOff, string Accent, string LeftStick, string RightStick);
    static readonly Look DualSense = new(Vertical("#CC243352", "#CC0E1526"), "#7FE3FF", B("#0F1729"), B("#8FE6FF"), "#3FE0FF", "#E04CFF", "#3DFF8F");
    static readonly Look Xbox360 = new(Vertical("#CC1C2C48", "#CC0B1222"), "#4FB6FF", B("#0D1628"), B("#7CC8FF"), "#3FA9FF", "#3FD0FF", "#3FD0FF");
    static readonly IBrush LabelOn = B("#04121C"); // dark text on a bright pressed fill

    readonly Look _look;
    readonly BoxShadows _glow;
    readonly Dictionary<string, (Border Box, TextBlock? Label, IBrush LabelOff, IBrush On, BoxShadows Glow)> _parts = new();
    Shape _l2 = null!, _r2 = null!;
    Ellipse _leftDot = null!, _rightDot = null!;
    Point _leftC, _rightC;
    readonly Canvas _canvas;

    public PadMap(Canvas canvas, bool xbox)
    {
        _canvas = canvas;
        _look = xbox ? Xbox360 : DualSense;
        _glow = Shadow(_look.Accent);
        canvas.Width = 260; canvas.Height = 170;

        Add(new Shape
        {
            Data = Geometry.Parse("M 40,24 C 40,18 55,18 75,18 L 185,18 C 205,18 220,18 220,24 C 236,70 252,128 234,150 C 220,166 196,162 186,142 L 176,122 L 84,122 L 74,142 C 64,162 40,166 26,150 C 8,128 24,70 40,24 Z"),
            Fill = _look.Body, Stroke = B(_look.Edge), StrokeThickness = 1.4,
            Effect = new DropShadowEffect { Color = Color.Parse(_look.Edge), BlurRadius = 12, OffsetX = 0, OffsetY = 0, Opacity = 0.7 },
        });
        // shoulders: curved trigger caps on the outer corners (fill with pull), bumper keys just inside them
        _l2 = Trigger("M 30,27 C 30,12 40,4 54,4 L 60,4 C 63,4 64,6 64,9 L 64,16 C 52,16 42,20 37,28 Z", 41, 6, xbox ? "LT" : "L2", "#3FE0FF");
        _r2 = Trigger("M 230,27 C 230,12 220,4 206,4 L 200,4 C 197,4 196,6 196,9 L 196,16 C 208,16 218,20 223,28 Z", 199, 6, xbox ? "RT" : "R2", "#FF7A3D");
        Part(xbox ? "LeftShoulder" : "L1", 68, 7, 24, 12, 4, xbox ? "LB" : "L1", 7, colour: "#4D7BFF");
        Part(xbox ? "RightShoulder" : "R1", 168, 7, 24, 12, 4, xbox ? "RB" : "R1", 7, colour: "#B44DFF");

        if (xbox)
        {
            _leftC = new Point(62, 56); _rightC = new Point(162, 104);
            Dpad(98, 104, "#3FD0FF", "#3FD0FF", "#3FD0FF", "#3FD0FF");
            Part("Back", 106, 52, 14, 9, 4, "◀", 5);
            Part("Start", 140, 52, 14, 9, 4, "▶", 5);
            Part("Guide", 120, 29, 20, 20, 10, "✕", 10);
            Part("Y", 189, 42, 18, 18, 9, "Y", 10, colour: "#FFC83D");
            Part("B", 207, 60, 18, 18, 9, "B", 10, colour: "#FF5C5C");
            Part("A", 189, 78, 18, 18, 9, "A", 10, colour: "#3DDC84");
            Part("X", 171, 60, 18, 18, 9, "X", 10, colour: "#4DA3FF");
        }
        else
        {
            _leftC = new Point(98, 104); _rightC = new Point(162, 104);
            Dpad(62, 66, "#9B6BFF", "#9B6BFF", "#FF4FA3", "#3DFF8F");
            Part("Touchpad", 100, 28, 60, 26, 6, null, 0);
            // light bars either side of the touchpad, in the two stick colours
            Add(new Border { Width = 2, Height = 24, CornerRadius = new CornerRadius(1), Background = B(_look.LeftStick), BoxShadow = Shadow(_look.LeftStick) }, 96, 29);
            Add(new Border { Width = 2, Height = 24, CornerRadius = new CornerRadius(1), Background = B(_look.RightStick), BoxShadow = Shadow(_look.RightStick) }, 162, 29);
            Part("Create", 84, 32, 8, 12, 4, null, 0);
            Part("Options", 168, 32, 8, 12, 4, null, 0);
            Part("Ps", 123, 66, 14, 14, 7, "PS", 5);
            Part("Mute", 124, 86, 12, 5, 2, null, 0);
            Part("Triangle", 189, 42, 18, 18, 9, "△", 10, colour: "#3DE8B0");
            Part("Circle", 207, 60, 18, 18, 9, "○", 10, colour: "#FF5C7A");
            Part("Cross", 189, 78, 18, 18, 9, "✕", 10, colour: "#6FA8FF");
            Part("Square", 171, 60, 18, 18, 9, "□", 10, colour: "#E07CFF");
        }
        // sticks: neon rings with radar lines (light up on L3/R3); the dot follows the stick
        Well(xbox ? "LeftThumb" : "L3", _leftC, _look.LeftStick);
        Well(xbox ? "RightThumb" : "R3", _rightC, _look.RightStick);
        _leftDot = Dot(_leftC, _look.LeftStick); _rightDot = Dot(_rightC, _look.RightStick);
    }

    static BoxShadows Shadow(string hex) => BoxShadows.Parse($"0 0 8 2 #AA{hex.TrimStart('#')}");

    void Add(Control c, double x = 0, double y = 0)
    {
        Canvas.SetLeft(c, x); Canvas.SetTop(c, y);
        _canvas.Children.Add(c);
    }

    void Dpad(double cx, double cy, string up, string down, string left, string right)
    {
        Part("DpadUp", cx - 7, cy - 21, 14, 14, 3, "▲", 7, colour: up);
        Part("DpadDown", cx - 7, cy + 7, 14, 14, 3, "▼", 7, colour: down);
        Part("DpadLeft", cx - 21, cy - 7, 14, 14, 3, "◀", 7, colour: left);
        Part("DpadRight", cx + 7, cy - 7, 14, 14, 3, "▶", 7, colour: right);
    }

    void Well(string key, Point c, string colour)
    {
        Part(key, c.X - 19, c.Y - 19, 38, 38, 19, null, 0, colour: colour, edgeWidth: 2);
        var line = B("#55" + colour.TrimStart('#'));
        Add(new Ellipse { Width = 24, Height = 24, Stroke = line, StrokeThickness = 0.6, IsHitTestVisible = false }, c.X - 12, c.Y - 12);
        Add(new Line { StartPoint = new Point(c.X - 16, c.Y), EndPoint = new Point(c.X + 16, c.Y), Stroke = line, StrokeThickness = 0.5, IsHitTestVisible = false });
        Add(new Line { StartPoint = new Point(c.X, c.Y - 16), EndPoint = new Point(c.X, c.Y + 16), Stroke = line, StrokeThickness = 0.5, IsHitTestVisible = false });
    }

    /// <summary>An outlined shoulder cap plus a glowing copy whose opacity follows the trigger pull.</summary>
    Shape Trigger(string geometry, double labelX, double labelY, string label, string colour)
    {
        Add(new Shape { Data = Geometry.Parse(geometry), Fill = _look.Off, Stroke = B(colour), StrokeThickness = 1.2 });
        var fill = new Shape
        {
            Data = Geometry.Parse(geometry), Fill = B(colour), Opacity = 0,
            Effect = new DropShadowEffect { Color = Color.Parse(colour), BlurRadius = 10, OffsetX = 0, OffsetY = 0 },
        };
        Add(fill);
        Add(new TextBlock { Text = label, FontSize = 6, FontWeight = FontWeight.SemiBold, Foreground = B(colour), IsHitTestVisible = false }, labelX, labelY);
        return fill;
    }

    /// <param name="colour">Own colour for outline, label and pressed glow; null uses the pad's accent.</param>
    void Part(string key, double x, double y, double w, double h, double r, string? label, double font,
              string? colour = null, double edgeWidth = 0.9)
    {
        var labelOff = colour is null ? _look.LabelOff : B(colour);
        TextBlock? tb = null;
        var box = new Border
        {
            Width = w, Height = h, CornerRadius = new CornerRadius(r),
            Background = _look.Off, BorderBrush = B(colour ?? _look.Edge), BorderThickness = new Thickness(edgeWidth),
        };
        if (label is not null)
        {
            tb = new TextBlock
            {
                Text = label, FontSize = font, FontWeight = FontWeight.SemiBold, Foreground = labelOff,
                HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center,
            };
            box.Child = tb;
        }
        Add(box, x, y);
        _parts[key] = (box, tb, labelOff, B(colour ?? _look.Accent), colour is null ? _glow : Shadow(colour));
    }

    Ellipse Dot(Point c, string colour)
    {
        var d = new Ellipse { Width = 9, Height = 9, Fill = B(colour), IsHitTestVisible = false };
        Add(d, c.X - 4.5, c.Y - 4.5);
        return d;
    }

    /// <summary>Sticks are raw 0-255 (centre 128); triggers 0-255.</summary>
    public void Update(Func<string, bool> pressed, byte l2, byte r2, byte lx, byte ly, byte rx, byte ry)
    {
        foreach (var (key, (box, label, labelOff, on, glow)) in _parts)
        {
            bool p = pressed(key);
            box.Background = p ? on : _look.Off;
            box.BoxShadow = p ? glow : default;
            if (label is not null) label.Foreground = p ? LabelOn : labelOff;
        }
        _l2.Opacity = l2 / 255.0; _r2.Opacity = r2 / 255.0;
        Move(_leftDot, _leftC, lx, ly); Move(_rightDot, _rightC, rx, ry);
    }

    static void Move(Ellipse dot, Point c, byte x, byte y)
    {
        // +-12px of travel inside the 38px well
        Canvas.SetLeft(dot, c.X - 4.5 + (x - 128) / 128.0 * 12);
        Canvas.SetTop(dot, c.Y - 4.5 + (y - 128) / 128.0 * 12);
    }
}
