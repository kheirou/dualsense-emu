using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;

namespace DualSenseEmu.Gui;

/// <summary>A vector gamepad drawn on a 260x170 canvas whose buttons light up. Two layouts: DualSense and Xbox.</summary>
public sealed class PadMap
{
    static readonly IBrush Off = new SolidColorBrush(Color.Parse("#12141A"));
    static readonly IBrush BodyFill = new SolidColorBrush(Color.Parse("#232733"));
    static readonly IBrush Edge = new SolidColorBrush(Color.Parse("#3A4050"));
    static readonly IBrush Dim = new SolidColorBrush(Color.Parse("#8A93A6"));
    static readonly IBrush Accent = new SolidColorBrush(Color.Parse("#6C8CFF"));

    readonly Dictionary<string, (Border Box, TextBlock? Label, IBrush On)> _parts = new();
    Border _l2 = null!, _r2 = null!;
    Ellipse _leftDot = null!, _rightDot = null!;
    Point _leftC, _rightC;
    readonly Canvas _canvas;

    public PadMap(Canvas canvas, bool xbox)
    {
        _canvas = canvas;
        canvas.Width = 260; canvas.Height = 170;
        // silhouette
        canvas.Children.Add(new Avalonia.Controls.Shapes.Path
        {
            Data = Geometry.Parse("M 40,24 C 40,18 55,18 75,18 L 185,18 C 205,18 220,18 220,24 C 236,70 252,128 234,150 C 220,166 196,162 186,142 L 176,122 L 84,122 L 74,142 C 64,162 40,166 26,150 C 8,128 24,70 40,24 Z"),
            Fill = BodyFill, Stroke = Edge, StrokeThickness = 2,
        });
        // triggers (fill brightens with pull) and bumpers
        _l2 = Trigger(36, 0); _r2 = Trigger(184, 0);
        Part(xbox ? "LeftShoulder" : "L1", 32, 9, 50, 11, 4, xbox ? "LB" : "L1", Accent, 8);
        Part(xbox ? "RightShoulder" : "R1", 178, 9, 50, 11, 4, xbox ? "RB" : "R1", Accent, 8);

        // face buttons (diamond), identical position on both pads
        IBrush Face(string xboxColor) => xbox ? new SolidColorBrush(Color.Parse(xboxColor)) : Accent;
        Part(xbox ? "Y" : "Triangle", 189, 42, 18, 18, 9, xbox ? "Y" : "△", Face("#FFC83D"), 10);
        Part(xbox ? "B" : "Circle", 207, 60, 18, 18, 9, xbox ? "B" : "○", Face("#FF5C5C"), 10);
        Part(xbox ? "A" : "Cross", 189, 78, 18, 18, 9, xbox ? "A" : "✕", Face("#3DDC84"), 10);
        Part(xbox ? "X" : "Square", 171, 60, 18, 18, 9, xbox ? "X" : "□", Face("#4DA3FF"), 10);

        if (xbox)
        {
            _leftC = new Point(62, 56); _rightC = new Point(162, 104);
            Dpad(98, 104);
            Part("Back", 108, 52, 12, 10, 5, null, Accent, 0);
            Part("Start", 140, 52, 12, 10, 5, null, Accent, 0);
            Part("Guide", 122, 32, 16, 16, 8, "◉", Accent, 8);
        }
        else
        {
            _leftC = new Point(98, 104); _rightC = new Point(162, 104);
            Dpad(62, 66);
            Part("Touchpad", 100, 28, 60, 26, 6, null, Accent, 0);
            Part("Create", 84, 32, 12, 8, 4, null, Accent, 0);
            Part("Options", 164, 32, 12, 8, 4, null, Accent, 0);
            Part("Ps", 123, 66, 14, 14, 7, "PS", Accent, 6);
            Part("Mute", 125, 88, 10, 6, 3, null, Accent, 0);
        }
        // sticks: ring lights on press (L3/R3), dot follows the stick
        string l3 = xbox ? "LeftThumb" : "L3", r3 = xbox ? "RightThumb" : "R3";
        Part(l3, _leftC.X - 19, _leftC.Y - 19, 38, 38, 19, null, Accent, 0);
        Part(r3, _rightC.X - 19, _rightC.Y - 19, 38, 38, 19, null, Accent, 0);
        _leftDot = Dot(_leftC); _rightDot = Dot(_rightC);
    }

    void Dpad(double cx, double cy)
    {
        Part("DpadUp", cx - 7, cy - 21, 14, 14, 3, "▲", Accent, 7);
        Part("DpadDown", cx - 7, cy + 7, 14, 14, 3, "▼", Accent, 7);
        Part("DpadLeft", cx - 21, cy - 7, 14, 14, 3, "◀", Accent, 7);
        Part("DpadRight", cx + 7, cy - 7, 14, 14, 3, "▶", Accent, 7);
    }

    Border Trigger(double x, double y)
    {
        var outline = new Border { Width = 40, Height = 9, CornerRadius = new CornerRadius(4), Background = Off, BorderBrush = Edge, BorderThickness = new Thickness(1) };
        Canvas.SetLeft(outline, x); Canvas.SetTop(outline, y); _canvas.Children.Add(outline);
        var fill = new Border { Width = 40, Height = 9, CornerRadius = new CornerRadius(4), Background = Accent, Opacity = 0 };
        Canvas.SetLeft(fill, x); Canvas.SetTop(fill, y); _canvas.Children.Add(fill);
        return fill;
    }

    void Part(string key, double x, double y, double w, double h, double r, string? label, IBrush on, double font)
    {
        TextBlock? tb = null;
        var box = new Border
        {
            Width = w, Height = h, CornerRadius = new CornerRadius(r),
            Background = Off, BorderBrush = Edge, BorderThickness = new Thickness(1),
        };
        if (label is not null)
        {
            tb = new TextBlock { Text = label, FontSize = font, Foreground = Dim, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
            box.Child = tb;
        }
        Canvas.SetLeft(box, x); Canvas.SetTop(box, y);
        _canvas.Children.Add(box);
        _parts[key] = (box, tb, on);
    }

    Ellipse Dot(Point c)
    {
        var d = new Ellipse { Width = 12, Height = 12, Fill = Accent, IsHitTestVisible = false };
        Canvas.SetLeft(d, c.X - 6); Canvas.SetTop(d, c.Y - 6);
        _canvas.Children.Add(d);
        return d;
    }

    /// <summary>Sticks are raw 0-255 (centre 128); triggers 0-255.</summary>
    public void Update(Func<string, bool> pressed, byte l2, byte r2, byte lx, byte ly, byte rx, byte ry)
    {
        foreach (var (key, (box, label, on)) in _parts)
        {
            bool p = pressed(key);
            box.Background = p ? on : Off;
            if (label is not null) label.Foreground = p ? Brushes.White : Dim;
        }
        _l2.Opacity = l2 / 255.0; _r2.Opacity = r2 / 255.0;
        Move(_leftDot, _leftC, lx, ly); Move(_rightDot, _rightC, rx, ry);
    }

    static void Move(Ellipse dot, Point c, byte x, byte y)
    {
        // +-9px of travel inside the 38px ring
        Canvas.SetLeft(dot, c.X - 6 + (x - 128) / 128.0 * 9);
        Canvas.SetTop(dot, c.Y - 6 + (y - 128) / 128.0 * 9);
    }
}
