// Records what the C# X360Mapper returns; rust/x360/tests/golden.jsonl is checked by `cargo test`.
// Run: dotnet fsi rust/x360/gen-golden.fsx   (after building src/DualSenseEmu.Core in Release)
#r "../../src/DualSenseEmu.Core/bin/Release/net9.0/DualSenseEmu.Core.dll"
open DualSenseEmu.Core
open System.IO
open System.Globalization
let f (d: float) = d.ToString("R", CultureInfo.InvariantCulture)
let w = new StreamWriter(Path.Combine(__SOURCE_DIRECTORY__, "tests", "golden", "numeric.jsonl"))
for v in 0 .. 255 do
    for inv in [false; true] do
        w.WriteLine(sprintf "{\"fn\":\"axis\",\"v\":%d,\"invert\":%b,\"out\":%d}" v inv (X360Mapper.Axis(byte v, inv)))
for v in 0 .. 255 do
    for t in [0; 1; 50; 127; 128; 200; 254; 255] do
        w.WriteLine(sprintf "{\"fn\":\"trigger\",\"v\":%d,\"t\":%d,\"out\":%d}" v t (X360Mapper.Trigger(byte v, byte t)))
let pts = [0; 1; 17; 64; 100; 127; 128; 129; 140; 160; 200; 254; 255]
for dz in [0.0; 0.05; 0.2; 0.5; 0.95] do
    for c in [0.05; 1.0; 1.7; 2.0; 3.5] do
        for inv in [false; true] do
            for x in pts do
                for y in pts do
                    let struct (ox, oy) = X360Mapper.Stick(byte x, byte y, dz, c, inv)
                    w.WriteLine(sprintf "{\"fn\":\"stick\",\"x\":%d,\"y\":%d,\"dz\":%s,\"curve\":%s,\"invert\":%b,\"ox\":%d,\"oy\":%d}" x y (f dz) (f c) inv ox oy)
w.Close()
