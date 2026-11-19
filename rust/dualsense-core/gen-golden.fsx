// Records what the C# Core returns; tests/golden/*.jsonl is replayed by `cargo test`.
// Run: dotnet fsi rust/dualsense-core/gen-golden.fsx   (after building src/DualSenseEmu.Core in Release)
#r "../../src/DualSenseEmu.Core/bin/Release/net9.0/DualSenseEmu.Core.dll"
open System
open System.IO
open System.Text.Json.Nodes
open DualSenseEmu.Core

let dir = Path.Combine(__SOURCE_DIRECTORY__, "tests", "golden")
let ji (x: int) : JsonNode = JsonValue.Create(x)
let jd (x: float) : JsonNode = JsonValue.Create(x)
let js (x: string) : JsonNode = if isNull x then null else JsonValue.Create(x)
let jb (x: bool) : JsonNode = JsonValue.Create(x)
let hex (b: byte[]) = Convert.ToHexString(b).ToLowerInvariant()
let line (kvs: (string * JsonNode) list) =
    let o = JsonObject()
    for (k, v) in kvs do o.Add(k, v)
    o.ToJsonString()
let rnd = Random(12345)
let rbytes n = Array.init n (fun _ -> byte (rnd.Next 256))
let rgbJ (c: Nullable<struct (byte * byte * byte)>) : JsonNode =
    if c.HasValue then
        let struct (a, b, d) = c.Value
        JsonArray(ji (int a), ji (int b), ji (int d)) :> JsonNode
    else null

let profiles =
    [ "{}"
      "{\"LeftDeadzone\":0.2}"
      "{\"Curve\":2,\"RightDeadzone\":0.1}"
      "{\"SwapSticks\":true,\"InvertLeftY\":true}"
      "{\"TriggerThreshold\":50}"
      "{\"Remap\":{\"Cross\":\"B\",\"Touchpad\":\"Guide\",\"Mute\":\"None\",\"Circle\":\"None\"}}"
      "{\"SwapSticks\":true,\"InvertRightY\":true,\"LeftDeadzone\":0.3,\"RightDeadzone\":0.05,\"Curve\":1.7,\"TriggerThreshold\":254,\"Remap\":{\"L1\":\"DpadUp\"}}"
      "not json"
      "{\"Curve\":\"abc\"}"
      "{\"LightbarMode\":\"Battery\",\"Lightbar\":\"#FF8800\",\"PlayerMode\":\"Battery\",\"PlayerNumber\":2}"
      "{\"LightbarMode\":\"Charging\",\"Lightbar\":\"00FF00\",\"PlayerMode\":\"Player\",\"PlayerNumber\":4}"
      "{\"PlayerMode\":\"Charging\",\"PlayerNumber\":9,\"Lightbar\":\"#12\"}"
      "{\"Unknown\":1,\"Curve\":0.05}" ]
let load (json: string) =
    let tmp = Path.GetTempFileName()
    File.WriteAllText(tmp, json)
    let p = MapProfile.Load tmp
    File.Delete tmp
    p

do
    use w = new StreamWriter(Path.Combine(dir, "profile.jsonl"))
    for pj in profiles do
        let p = load pj
        let remap = JsonObject()
        if not (isNull p.Remap) then
            for kv in p.Remap |> Seq.sortBy (fun kv -> int kv.Key) do remap.Add(string kv.Key, js (string kv.Value))
        w.WriteLine(line [ "json", js pj; "left_deadzone", jd p.LeftDeadzone; "right_deadzone", jd p.RightDeadzone
                           "curve", jd p.Curve; "invert_left_y", jb p.InvertLeftY; "invert_right_y", jb p.InvertRightY
                           "swap_sticks", jb p.SwapSticks; "trigger_threshold", ji (int p.TriggerThreshold)
                           "lightbar", js p.Lightbar; "lightbar_mode", js (string p.LightbarMode)
                           "player_mode", js (string p.PlayerMode); "player_number", ji (int p.PlayerNumber)
                           "remap", (if isNull p.Remap then null else remap :> JsonNode) ])

do
    use w = new StreamWriter(Path.Combine(dir, "parser.jsonl"))
    let emit (r: byte[]) =
        let mutable s = Unchecked.defaultof<DualSenseState>
        let ok = ReportParser.TryParse(ReadOnlySpan r, &s)
        w.WriteLine(line [ "report", js (hex r); "ok", jb ok
                           "lx", ji (int s.Lx); "ly", ji (int s.Ly); "rx", ji (int s.Rx); "ry", ji (int s.Ry)
                           "l2", ji (int s.L2); "r2", ji (int s.R2); "buttons", ji (int s.Buttons)
                           "battery", ji s.BatteryPercent; "charge", js (string s.Charge) ])
    for id in [ 0uy; 1uy; 2uy; 0x31uy; 0xFFuy ] do
        for len in 0 .. 80 do
            for _ in 1 .. 3 do
                let r = rbytes len
                if len > 0 then r.[0] <- id
                emit r
    for id, off in [ 1uy, 1; 0x31uy, 2 ] do
        for hat in 0 .. 15 do
            for bits in 0 .. 255 do
                let r = rbytes 64
                r.[0] <- id
                r.[off + 7] <- byte ((bits &&& 0xF0) ||| hat)
                emit r
    for st in 0 .. 255 do
        let r = rbytes 64
        r.[0] <- 1uy
        r.[53] <- byte st
        emit r

do
    use w = new StreamWriter(Path.Combine(dir, "map.jsonl"))
    let states =
        [ for _ in 1 .. 250 do
              yield DualSenseState(byte (rnd.Next 256), byte (rnd.Next 256), byte (rnd.Next 256), byte (rnd.Next 256),
                             byte (rnd.Next 256), byte (rnd.Next 256), LanguagePrimitives.EnumOfValue<uint32, DsButtons>(uint32 (rnd.Next(1 <<< 17))))
          for v in [ 0; 1; 127; 128; 129; 254; 255 ] do
              yield DualSenseState(byte v, byte (255 - v), byte v, byte v, byte v, byte (255 - v), DsButtons.None) ]
    for pj in profiles do
        let p = load pj
        for s in states do
            let x = X360Mapper.Map(s, p)
            w.WriteLine(line [ "profile", js pj; "lx", ji (int s.Lx); "ly", ji (int s.Ly); "rx", ji (int s.Rx); "ry", ji (int s.Ry)
                               "l2", ji (int s.L2); "r2", ji (int s.R2); "buttons", ji (int s.Buttons)
                               "o_buttons", ji (int x.Buttons); "o_lt", ji (int x.LeftTrigger); "o_rt", ji (int x.RightTrigger)
                               "o_lx", ji (int x.LeftX); "o_ly", ji (int x.LeftY); "o_rx", ji (int x.RightX); "o_ry", ji (int x.RightY) ])
    for b in [ DsButtons.None; DsButtons.Cross; DsButtons.Touchpad; DsButtons.Mute; DsButtons.Ps; DsButtons.L3; DsButtons.DpadLeft ] do
        let d = X360Mapper.DefaultFor b
        w.WriteLine(line [ "default_for", ji (int b); "out", (if d.HasValue then ji (int d.Value) else null) ])

do
    use w = new StreamWriter(Path.Combine(dir, "output.jsonl"))
    for bt in [ false; true ] do
        for _ in 1 .. 300 do
            let rgb : Nullable<struct (byte * byte * byte)> = if rnd.Next 2 = 0 then Nullable() else Nullable(struct (byte (rnd.Next 256), byte (rnd.Next 256), byte (rnd.Next 256)))
            let leds : Nullable<byte> = if rnd.Next 2 = 0 then Nullable() else Nullable(byte (rnd.Next 256))
            let l, r, seq = byte (rnd.Next 256), byte (rnd.Next 256), byte (rnd.Next 256)
            let out = OutputReport.Build(bt, l, r, rgb, seq, leds)
            w.WriteLine(line [ "fn", js "build"; "bt", jb bt; "left", ji (int l); "right", ji (int r); "rgb", rgbJ rgb; "seq", ji (int seq)
                               "leds", (if leds.HasValue then ji (int leds.Value) else null); "out", js (hex out) ])
    for len in 0 .. 60 do
        let b = rbytes len
        w.WriteLine(line [ "fn", js "crc32"; "body", js (hex b); "out", ji (int (OutputReport.Crc32(ReadOnlySpan b))) ])
    for h in [ "#FF8800"; "FF8800"; "##FF8800"; "#ff8800"; "#GG0000"; "#FFF"; ""; "#1234567"; " FF880"; "FF880 "; "\t0A0B0C"; "#12 456"; "+12345"; "-12345"; "#" ] do
        w.WriteLine(line [ "fn", js "colour"; "hex", js h; "out", rgbJ (OutputReport.ParseColour h) ])
    w.WriteLine(line [ "fn", js "colour"; "hex", null; "out", null ])
    for pj in profiles do
        let p = load pj
        let states =
            [ yield None
              for pct in [ -1; 0; 10; 19; 20; 39; 40; 99; 100 ] do
                  for c in [ ChargeState.Unknown; ChargeState.Discharging; ChargeState.Charging; ChargeState.Full ] ->
                      Some(DualSenseState(128uy, 128uy, 128uy, 128uy, 0uy, 0uy, DsButtons.None, pct, c)) ]
        for so in states do
            let arg : Nullable<DualSenseState> = match so with Some s -> Nullable s | None -> Nullable()
            let c = OutputReport.Resolve(p, arg)
            let l = OutputReport.ResolveLeds(p, arg)
            w.WriteLine(line [ "fn", js "resolve"; "profile", js pj
                               "battery", (match so with Some s -> ji s.BatteryPercent | None -> null)
                               "charge", (match so with Some s -> js (string s.Charge) | None -> null)
                               "colour", rgbJ c; "leds", (if l.HasValue then ji (int l.Value) else null) ])
