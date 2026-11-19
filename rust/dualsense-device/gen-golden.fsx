// Records what the C# Device returns for HidHide and StatusFile; tests/golden/*.jsonl is replayed by `cargo test`.
// Run: dotnet fsi rust/dualsense-device/gen-golden.fsx   (after `dotnet build -c Release` of src/DualSenseEmu.Device)
#r "../../src/DualSenseEmu.Core/bin/Release/net9.0/DualSenseEmu.Core.dll"
#r "../../src/DualSenseEmu.Device/bin/Release/net9.0/DualSenseEmu.Device.dll"
open System
open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open DualSenseEmu.Core
open DualSenseEmu.Device

let dir = Path.Combine(__SOURCE_DIRECTORY__, "tests", "golden")
Directory.CreateDirectory dir |> ignore
let js (x: string) : JsonNode = if isNull x then null else JsonValue.Create(x)
let jb (x: bool) : JsonNode = JsonValue.Create(x)
let line (kvs: (string * JsonNode) list) =
    let o = JsonObject()
    for (k, v) in kvs do o.Add(k, v)
    o.ToJsonString()
let arr (xs: string seq) : JsonNode =
    let a = JsonArray()
    for x in xs do a.Add(JsonValue.Create(x))
    a :> JsonNode

let sample = """
[ { "friendlyName" : "Controller (XBOX 360 For Windows)" , "devices" : [
{ "present" : true , "deviceInstancePath" : "HID\\VID_045E&PID_028E&IG_00\\3&34f4d81e&0&0000" } ] } ,
{ "friendlyName" : "Sony Interactive Entertainment DualSense Wireless Controller" , "devices" : [
{ "present" : true , "deviceInstancePath" : "HID\\{00001124-0000-1000-8000-00805f9b34fb}_VID&0002054c_PID&0ce6\\8&2c76ced&0&0000" } ] } ,
{ "friendlyName" : "Wireless Controller" , "devices" : [
{ "present" : true , "deviceInstancePath" : "HID\\VID_054C&PID_0DF2\\7&1&0&0000" } ] } ]
"""
let dev (p: string) = "{\"deviceInstancePath\":" + JsonSerializer.Serialize p + "}"
let wrap (devs: string list) = "[{\"devices\":[" + String.Join(",", devs) + "]}]"

do
    use w = new StreamWriter(Path.Combine(dir, "hidhide.jsonl"))
    let inputs =
        [ sample; "[]"; ""; "HidHide driver not found"; "[{}]"; "[{\"devices\":[]}]"; "[{\"devices\":[{}]}]"
          wrap [ dev "HID\\VID_054C&PID_0CE6\\1\" & calc & \"" ]
          wrap [ dev "HID\\VID_054C&PID_0CE6\\1 2" ]
          wrap [ dev "HID\\VID_054C&PID_0CE6\\%PATH%" ]
          wrap [ dev "hid\\vid_054c&pid_0ce6\\a"; dev "HID\\VID_054C&PID_0CE6\\A"; dev "hid\\vid_054c&pid_0ce6\\a" ]
          wrap [ dev "HID\\VID_054C&PID_0DF2\\9"; dev "HID\\VID_054C&PID_0DF2\\9" ]
          wrap [ dev "HID\\VID_054C&PID_0000\\9" ]
          wrap [ dev "HID\\VID_054D&PID_0CE6\\9" ]
          wrap [ dev "HID\\VID_054C_PID_0CE6\\9" ] ]
    for i in inputs do
        w.WriteLine(line [ "fn", js "parse"; "json", js i; "out", arr (HidHide.ParseDualSensePaths i) ])
    w.WriteLine(line [ "fn", js "cli_path"; "out", js HidHide.CliPath ])
    for hide in [ true; false ] do
        for pads in [ []; [ "HID\\A\\1" ]; [ "HID\\A\\1"; "HID\\B\\2" ] ] do
            for app in [ @"C:\Apps\DualSenseEmu.exe"; @"C:\Program Files\x\y.exe"; "bad\"quote.exe"; "bad%pct.exe" ] do
                let out, err =
                    try HidHide.BuildChain(hide, pads, app), false
                    with :? ArgumentException -> null, true
                w.WriteLine(line [ "fn", js "chain"; "hide", jb hide; "pads", arr pads; "app", js app; "out", js out; "error", jb err ])

do
    use w = new StreamWriter(Path.Combine(dir, "status.jsonl"))
    let dsList =
        [ None
          for pct in [ -1; 0; 55; 100 ] do
              for c in [ ChargeState.Unknown; ChargeState.Discharging; ChargeState.Charging; ChargeState.Full ] do
                  yield Some(DualSenseState(128uy, 128uy, 128uy, 128uy, 0uy, 0uy, DsButtons.None, pct, c)) ]
    for st in [ EmuState.Stopped; EmuState.Waiting; EmuState.Running; EmuState.Error; EmuState.DriverMissing ] do
        for tr in [ Transport.None; Transport.Usb; Transport.Bluetooth ] do
            for d in dsList do
                let arg : Nullable<DualSenseState> = match d with Some s -> Nullable s | None -> Nullable()
                w.WriteLine(line [ "state", js (string st); "transport", js (string tr)
                                   "battery", (match d with Some s -> JsonValue.Create(s.BatteryPercent) :> JsonNode | None -> null)
                                   "charge", (match d with Some s -> js (string s.Charge) | None -> null)
                                   "out", js (StatusFile.ToJson(st, tr, arg)) ])
