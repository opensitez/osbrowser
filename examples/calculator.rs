//! A complete, fancy wide Scientific & Programmer Calculator running in the system browser
//! using osbrowser's WHATWG webcore-compatible DOM and event API.
//!
//! Run with:
//!     cargo run --manifest-path osbrowser/Cargo.toml --example calculator

use std::f64::consts::{E, PI};
use std::sync::{Arc, Mutex};
use osbrowser::browser::{Browser, BrowserDocument, BrowserGuest, BrowserOptions};
use osbrowser::dom::events::ListenerOptions;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Standard,
    Scientific,
    Programmer,
}

#[derive(Clone)]
struct CalcState {
    mode: Mode,
    display: String,
    history: String,
    stored: Option<f64>,
    op: Option<String>,
    reset_on_next: bool,
    rad_mode: bool,
}

impl CalcState {
    fn new() -> Self {
        Self {
            mode: Mode::Scientific, // Start in fancy wide mode
            display: "0".to_string(),
            history: String::new(),
            stored: None,
            op: None,
            reset_on_next: false,
            rad_mode: true,
        }
    }

    fn current_val(&self) -> f64 {
        if self.mode == Mode::Programmer && (self.display.starts_with("0x") || self.display.starts_with("0X")) {
            i64::from_str_radix(&self.display[2..], 16).unwrap_or(0) as f64
        } else {
            self.display.parse().unwrap_or(0.0)
        }
    }

    fn input_digit(&mut self, digit: char) {
        if self.reset_on_next || self.display == "0" {
            self.display = digit.to_string();
            self.reset_on_next = false;
        } else if self.display.len() < 24 {
            self.display.push(digit);
        }
    }

    fn input_hex_digit(&mut self, digit: char) {
        if self.mode != Mode::Programmer {
            return;
        }
        if self.reset_on_next || self.display == "0" {
            self.display = digit.to_string();
            self.reset_on_next = false;
        } else if self.display.len() < 16 {
            self.display.push(digit);
        }
    }

    fn input_dot(&mut self) {
        if self.mode == Mode::Programmer {
            return; // No decimals in programmer integer mode
        }
        if self.reset_on_next {
            self.display = "0.".to_string();
            self.reset_on_next = false;
        } else if !self.display.contains('.') {
            self.display.push('.');
        }
    }

    fn backspace(&mut self) {
        if self.reset_on_next {
            return;
        }
        if self.display.len() > 1 {
            self.display.pop();
        } else {
            self.display = "0".to_string();
        }
    }

    fn input_op(&mut self, op: &str) {
        let val = self.current_val();
        if let (Some(prev), Some(pending_op)) = (self.stored, &self.op) {
            let res = execute_binary_op(prev, val, pending_op);
            self.display = format_number(res);
            self.stored = Some(res);
            self.history = format!("{} {} {}", format_number(res), op_symbol(op), "");
        } else {
            self.stored = Some(val);
            self.history = format!("{} {}", format_number(val), op_symbol(op));
        }
        self.op = Some(op.to_string());
        self.reset_on_next = true;
    }

    fn calculate(&mut self) {
        if let (Some(prev), Some(op)) = (self.stored.take(), self.op.take()) {
            let val = self.current_val();
            let res = execute_binary_op(prev, val, &op);
            self.history = format!("{} {} {} =", format_number(prev), op_symbol(&op), format_number(val));
            self.display = format_number(res);
            self.reset_on_next = true;
        }
    }

    fn scientific_fn(&mut self, func: &str) {
        let val = self.current_val();
        let res = match func {
            "sin" => if self.rad_mode { val.sin() } else { val.to_radians().sin() },
            "cos" => if self.rad_mode { val.cos() } else { val.to_radians().cos() },
            "tan" => if self.rad_mode { val.tan() } else { val.to_radians().tan() },
            "ln" => if val > 0.0 { val.ln() } else { f64::NAN },
            "log" => if val > 0.0 { val.log10() } else { f64::NAN },
            "sqrt" => if val >= 0.0 { val.sqrt() } else { f64::NAN },
            "sqr" => val * val,
            "cube" => val * val * val,
            "inv" => if val != 0.0 { 1.0 / val } else { f64::NAN },
            "abs" => val.abs(),
            "fact" => factorial(val),
            "pi" => PI,
            "e" => E,
            _ => val,
        };
        self.history = format!("{}({})", func, format_number(val));
        self.display = format_number(res);
        self.reset_on_next = true;
    }

    fn bitwise_unary(&mut self, func: &str) {
        let val = self.current_val() as i64;
        let res = match func {
            "not" => !val,
            _ => val,
        };
        self.history = format!("{}({})", func.to_uppercase(), val);
        self.display = format!("{}", res);
        self.reset_on_next = true;
    }

    fn clear(&mut self) {
        self.display = "0".to_string();
        self.history.clear();
        self.stored = None;
        self.op = None;
        self.reset_on_next = false;
    }

    fn toggle_sign(&mut self) {
        if self.display != "0" {
            if self.display.starts_with('-') {
                self.display.remove(0);
            } else {
                self.display.insert(0, '-');
            }
        }
    }

    fn percent(&mut self) {
        let val = self.current_val();
        self.display = format_number(val / 100.0);
        self.reset_on_next = true;
    }

    fn get_programmer_readouts(&self) -> (String, String, String, String) {
        let val = self.current_val() as i64;
        let uval = val as u64;
        let hex = format!("0x{:08X}", uval);
        let dec = format!("{}", val);
        let oct = format!("0o{:011o}", uval);
        let bin_raw = format!("{:032b}", uval);
        let bin = bin_raw
            .as_bytes()
            .chunks(4)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        (hex, dec, oct, bin)
    }
}

fn execute_binary_op(a: f64, b: f64, op: &str) -> f64 {
    match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => if b == 0.0 { f64::NAN } else { a / b },
        "pow" => a.powf(b),
        "mod" => if b == 0.0 { f64::NAN } else { ((a as i64) % (b as i64)) as f64 },
        "and" => ((a as i64) & (b as i64)) as f64,
        "or" => ((a as i64) | (b as i64)) as f64,
        "xor" => ((a as i64) ^ (b as i64)) as f64,
        "lsh" => ((a as i64) << ((b as i64).clamp(0, 63))) as f64,
        "rsh" => ((a as i64) >> ((b as i64).clamp(0, 63))) as f64,
        _ => b,
    }
}

fn op_symbol(op: &str) -> &str {
    match op {
        "+" => "+",
        "-" => "−",
        "*" => "×",
        "/" => "÷",
        "pow" => "^",
        "mod" => "MOD",
        "and" => "AND",
        "or" => "OR",
        "xor" => "XOR",
        "lsh" => "<<",
        "rsh" => ">>",
        _ => op,
    }
}

fn factorial(n: f64) -> f64 {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return f64::NAN;
    }
    let mut acc = 1.0;
    for i in 1..=(n as u64) {
        acc *= i as f64;
    }
    acc
}

fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "Error".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    if n.fract() == 0.0 && n.abs() < 1e14 {
        format!("{:.0}", n)
    } else {
        let s = format!("{:.10}", n);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

#[derive(Clone, Copy)]
struct UIHandles {
    calc_card: u32,
    screen: u32,
    history: u32,
    r_hex: u32,
    r_dec: u32,
    r_oct: u32,
    r_bin: u32,
    tab_std: u32,
    tab_sci: u32,
    tab_prog: u32,
}

fn update_ui(doc: &mut osbrowser::types::Document, ui: UIHandles, s: &CalcState) {
    doc.set_text_content(ui.screen, &s.display);
    doc.set_text_content(ui.history, &s.history);
    let (h, d, o, b) = s.get_programmer_readouts();
    doc.set_text_content(ui.r_hex, &h);
    doc.set_text_content(ui.r_dec, &d);
    doc.set_text_content(ui.r_oct, &o);
    doc.set_text_content(ui.r_bin, &b);
}

struct Page;

impl BrowserGuest for Page {
    fn dispatch_pending(&mut self) -> bool {
        false
    }
}

fn main() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>osbrowser - Advanced Scientific & Programmer Calculator</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; user-select: none; }
    body {
      background: radial-gradient(circle at 50% 30%, #1e293b 0%, #030712 100%);
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Roboto, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      padding: 24px;
    }
    .calculator {
      background: rgba(15, 23, 42, 0.94);
      border-radius: 28px;
      padding: 24px;
      box-shadow: 0 25px 60px rgba(0, 0, 0, 0.8), 0 0 0 1px rgba(255, 255, 255, 0.1);
      backdrop-filter: blur(24px);
      transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    }
    .mode-standard { width: 380px; }
    .mode-scientific { width: 760px; }
    .mode-programmer { width: 800px; }

    /* Mode Switcher Tabs */
    .tab-bar {
      display: flex;
      background: rgba(2, 6, 23, 0.6);
      padding: 4px;
      border-radius: 14px;
      margin-bottom: 20px;
      gap: 6px;
      border: 1px solid rgba(255, 255, 255, 0.06);
    }
    .tab-btn {
      flex: 1;
      height: 38px;
      border-radius: 10px;
      border: none;
      background: transparent;
      color: #94a3b8;
      font-size: 0.95rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .tab-btn.active {
      background: #38bdf8;
      color: #030712;
      font-weight: 600;
      box-shadow: 0 4px 12px rgba(56, 189, 248, 0.35);
    }

    /* Screen display */
    .display-box {
      background: rgba(2, 6, 23, 0.8);
      border-radius: 18px;
      padding: 16px 20px;
      margin-bottom: 20px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      min-height: 105px;
      justify-content: center;
    }
    .history {
      color: #64748b;
      font-size: 1.05rem;
      font-weight: 400;
      min-height: 22px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      margin-bottom: 4px;
    }
    .display {
      color: #f8fafc;
      font-size: 3.2rem;
      font-weight: 300;
      letter-spacing: -1px;
      max-width: 100%;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    /* Programmer Multi-Base Readouts */
    .prog-readout {
      display: flex;
      flex-direction: column;
      gap: 4px;
      padding: 12px 16px;
      background: rgba(2, 6, 23, 0.6);
      border-radius: 14px;
      margin-bottom: 18px;
      font-family: "Courier New", monospace;
      font-size: 0.88rem;
      border: 1px solid rgba(255, 255, 255, 0.05);
    }
    .base-row {
      display: flex;
      justify-content: space-between;
    }
    .base-label { color: #38bdf8; font-weight: 600; width: 45px; }
    .base-val { color: #cbd5e1; letter-spacing: 0.5px; }

    /* Button grids */
    .grid {
      display: grid;
      gap: 10px;
    }
    .mode-standard .grid {
      grid-template-columns: repeat(4, 1fr);
    }
    .mode-scientific .grid {
      grid-template-columns: repeat(6, 1fr);
    }
    .mode-programmer .grid {
      grid-template-columns: repeat(6, 1fr);
    }

    button {
      height: 54px;
      border-radius: 16px;
      border: none;
      font-size: 1.25rem;
      font-weight: 500;
      cursor: pointer;
      display: flex;
      justify-content: center;
      align-items: center;
      transition: filter 0.12s ease, transform 0.08s ease, background 0.15s ease;
      outline: none;
    }
    button:active {
      transform: scale(0.95);
      filter: brightness(1.3);
    }
    .btn-num {
      background: #1e293b;
      color: #f8fafc;
    }
    .btn-num:hover { background: #334155; }
    .btn-op {
      background: #f59e0b;
      color: #030712;
      font-size: 1.4rem;
      font-weight: 600;
    }
    .btn-op:hover { background: #fbbf24; }
    .btn-fn {
      background: #334155;
      color: #94a3b8;
      font-size: 1.1rem;
    }
    .btn-fn:hover { background: #475569; color: #f8fafc; }
    .btn-sci {
      background: #0f172a;
      color: #38bdf8;
      font-size: 1.05rem;
      font-weight: 600;
      border: 1px solid rgba(56, 189, 248, 0.2);
    }
    .btn-sci:hover { background: #1e293b; color: #7dd3fc; }
    .btn-prog {
      background: #0f172a;
      color: #a855f7;
      font-size: 1.05rem;
      font-weight: 600;
      border: 1px solid rgba(168, 85, 247, 0.2);
    }
    .btn-prog:hover { background: #1e293b; color: #c084fc; }

    /* Conditional visibility per mode */
    .mode-standard .sci-key, .mode-standard .prog-key, .mode-standard .prog-readout { display: none !important; }
    .mode-scientific .prog-key, .mode-scientific .prog-readout { display: none !important; }
    .mode-programmer .sci-key { display: none !important; }
  </style>
</head>
<body>
  <div id="calc_card" class="calculator mode-scientific">
    <div class="tab-bar">
      <button id="tab_std" class="tab-btn">Standard</button>
      <button id="tab_sci" class="tab-btn active">Scientific</button>
      <button id="tab_prog" class="tab-btn">Programmer</button>
    </div>
    <div class="display-box">
      <div id="history" class="history"></div>
      <div id="screen" class="display">0</div>
    </div>
    <div id="prog_readout" class="prog-readout">
      <div class="base-row"><span class="base-label">HEX</span><span id="readout_hex" class="base-val">0x00000000</span></div>
      <div class="base-row"><span class="base-label">DEC</span><span id="readout_dec" class="base-val">0</span></div>
      <div class="base-row"><span class="base-label">OCT</span><span id="readout_oct" class="base-val">0o00000000000</span></div>
      <div class="base-row"><span class="base-label">BIN</span><span id="readout_bin" class="base-val">0000 0000 0000 0000 0000 0000 0000 0000</span></div>
    </div>
    <div id="keys" class="grid"></div>
  </div>
</body>
</html>"#;

    let document = BrowserDocument::new(osbrowser::load_html(html, 800.0));
    let state = Arc::new(Mutex::new(CalcState::new()));

    {
        let mut doc = document.write();
        let ui = UIHandles {
            calc_card: doc.query_selector("#calc_card").expect("calc_card exists"),
            screen: doc.query_selector("#screen").expect("screen exists"),
            history: doc.query_selector("#history").expect("history exists"),
            r_hex: doc.query_selector("#readout_hex").expect("readout_hex exists"),
            r_dec: doc.query_selector("#readout_dec").expect("readout_dec exists"),
            r_oct: doc.query_selector("#readout_oct").expect("readout_oct exists"),
            r_bin: doc.query_selector("#readout_bin").expect("readout_bin exists"),
            tab_std: doc.query_selector("#tab_std").expect("tab_std exists"),
            tab_sci: doc.query_selector("#tab_sci").expect("tab_sci exists"),
            tab_prog: doc.query_selector("#tab_prog").expect("tab_prog exists"),
        };
        let keys_container = doc.query_selector("#keys").expect("keys grid exists");

        // Tab Switching Listeners
        for &(tab_node, mode, mode_class) in &[
            (ui.tab_std, Mode::Standard, "calculator mode-standard"),
            (ui.tab_sci, Mode::Scientific, "calculator mode-scientific"),
            (ui.tab_prog, Mode::Programmer, "calculator mode-programmer"),
        ] {
            let s_clone = Arc::clone(&state);
            doc.add_event_listener(
                tab_node,
                "click",
                Box::new(move |_evt, doc| {
                    let mut s = s_clone.lock().unwrap();
                    s.mode = mode;
                    doc.set_attribute(ui.calc_card, "class", mode_class);
                    doc.set_attribute(ui.tab_std, "class", if mode == Mode::Standard { "tab-btn active" } else { "tab-btn" });
                    doc.set_attribute(ui.tab_sci, "class", if mode == Mode::Scientific { "tab-btn active" } else { "tab-btn" });
                    doc.set_attribute(ui.tab_prog, "class", if mode == Mode::Programmer { "tab-btn active" } else { "tab-btn" });
                    update_ui(doc, ui, &s);
                }),
                ListenerOptions::default(),
            );
        }

        // Full comprehensive button matrix:
        // (Label, CSS classes, action command)
        let all_buttons: &[(&str, &str, &str)] = &[
            // Row 1
            ("sin", "btn-sci sci-key", "fn:sin"),
            ("cos", "btn-sci sci-key", "fn:cos"),
            ("AND", "btn-prog prog-key", "op:and"),
            ("OR", "btn-prog prog-key", "op:or"),
            ("C", "btn-fn", "clear"),
            ("⌫", "btn-fn", "backspace"),
            ("+/-", "btn-fn", "sign"),
            ("÷", "btn-op", "op:/"),

            // Row 2
            ("tan", "btn-sci sci-key", "fn:tan"),
            ("x²", "btn-sci sci-key", "fn:sqr"),
            ("XOR", "btn-prog prog-key", "op:xor"),
            ("NOT", "btn-prog prog-key", "not"),
            ("7", "btn-num", "digit:7"),
            ("8", "btn-num", "digit:8"),
            ("9", "btn-num", "digit:9"),
            ("×", "btn-op", "op:*"),

            // Row 3
            ("ln", "btn-sci sci-key", "fn:ln"),
            ("log", "btn-sci sci-key", "fn:log"),
            ("<<", "btn-prog prog-key", "op:lsh"),
            (">>", "btn-prog prog-key", "op:rsh"),
            ("4", "btn-num", "digit:4"),
            ("5", "btn-num", "digit:5"),
            ("6", "btn-num", "digit:6"),
            ("−", "btn-op", "op:-"),

            // Row 4
            ("√x", "btn-sci sci-key", "fn:sqrt"),
            ("xʸ", "btn-sci sci-key", "op:pow"),
            ("MOD", "btn-prog prog-key", "op:mod"),
            ("A", "btn-prog prog-key", "hex:A"),
            ("1", "btn-num", "digit:1"),
            ("2", "btn-num", "digit:2"),
            ("3", "btn-num", "digit:3"),
            ("+", "btn-op", "op:+"),

            // Row 5
            ("1/x", "btn-sci sci-key", "fn:inv"),
            ("x!", "btn-sci sci-key", "fn:fact"),
            ("B", "btn-prog prog-key", "hex:B"),
            ("C", "btn-prog prog-key", "hex:C"),
            ("%", "btn-fn", "percent"),
            ("0", "btn-num", "digit:0"),
            (".", "btn-num", "dot"),
            ("=", "btn-op", "eq"),

            // Row 6 (Extra hex keys for programmer mode & constants for scientific)
            ("π", "btn-sci sci-key", "fn:pi"),
            ("e", "btn-sci sci-key", "fn:e"),
            ("D", "btn-prog prog-key", "hex:D"),
            ("E", "btn-prog prog-key", "hex:E"),
            ("F", "btn-prog prog-key", "hex:F"),
            ("|x|", "btn-sci sci-key", "fn:abs"),
        ];

        for &(label, class, action) in all_buttons {
            let btn = doc.create_element("button");
            doc.set_attribute(btn, "class", class);
            doc.set_text_content(btn, label);
            doc.append_child(keys_container, btn);

            let s_clone = Arc::clone(&state);
            let act = action.to_string();

            doc.add_event_listener(
                btn,
                "click",
                Box::new(move |_evt, doc| {
                    let mut s = s_clone.lock().unwrap();
                    if let Some(digit) = act.strip_prefix("digit:") {
                        s.input_digit(digit.chars().next().unwrap());
                    } else if let Some(hex) = act.strip_prefix("hex:") {
                        s.input_hex_digit(hex.chars().next().unwrap());
                    } else if let Some(op) = act.strip_prefix("op:") {
                        s.input_op(op);
                    } else if let Some(func) = act.strip_prefix("fn:") {
                        s.scientific_fn(func);
                    } else {
                        match act.as_str() {
                            "clear" => s.clear(),
                            "backspace" => s.backspace(),
                            "sign" => s.toggle_sign(),
                            "percent" => s.percent(),
                            "not" => s.bitwise_unary("not"),
                            "dot" => s.input_dot(),
                            "eq" => s.calculate(),
                            _ => {}
                        }
                    }
                    update_ui(doc, ui, &s);
                }),
                ListenerOptions::default(),
            );
        }
    }

    let mut browser = Browser::open(BrowserOptions {
        title: "osbrowser - Scientific & Programmer Calculator".into(),
        width: 840.0,
        height: 720.0,
        ..Default::default()
    })
    .expect("open browser");

    browser.attach_document(document);
    browser.set_guest(Page);
    println!("Advanced Calculator running in system browser tab. Close window or Ctrl+C to exit.");
    browser.run().expect("run browser");
}
