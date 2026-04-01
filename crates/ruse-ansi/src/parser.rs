/// ANSI escape sequence parser state machine.
///
/// Implements parsing for CSI, OSC, DCS, APC, PM, and SOS escape sequences
/// per the DEC VT specification. Bytes are fed one at a time via [`Parser::advance`],
/// and decoded events are dispatched to a [`Handler`] implementor.
const MAX_PARAMS: usize = 32;
const MAX_DATA: usize = 64 * 1024;

/// Callback trait for parsed ANSI events.
///
/// All methods have default no-op implementations so consumers can override
/// only the events they care about.
#[allow(unused_variables)]
pub trait Handler {
    /// A printable character was received in the ground state.
    fn print(&mut self, ch: char) {}

    /// A C0 or C1 control byte was received (e.g. BEL, BS, LF).
    fn execute(&mut self, byte: u8) {}

    /// A CSI (Control Sequence Introducer) sequence was completed.
    fn csi_dispatch(&mut self, params: &[i32], intermediates: &[u8], final_byte: u8) {}

    /// An ESC sequence was completed (non-CSI).
    fn esc_dispatch(&mut self, intermediates: &[u8], final_byte: u8) {}

    /// An OSC (Operating System Command) string was completed.
    fn osc_dispatch(&mut self, data: &[u8]) {}

    /// A DCS (Device Control String) was completed.
    fn dcs_dispatch(&mut self, params: &[i32], intermediates: &[u8], data: &[u8]) {}

    /// An APC (Application Program Command) string was completed.
    fn apc_dispatch(&mut self, data: &[u8]) {}

    /// A PM (Privacy Message) string was completed.
    fn pm_dispatch(&mut self, data: &[u8]) {}

    /// An SOS (Start of String) was completed.
    fn sos_dispatch(&mut self, data: &[u8]) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsString,
    OscString,
    SosString,
    PmString,
    ApcString,
    Utf8,
}

/// Byte-oriented ANSI escape sequence parser.
///
/// Feed bytes via [`advance`](Parser::advance) or [`process`](Parser::process);
/// decoded events are dispatched to the provided [`Handler`].
pub struct Parser {
    state: State,
    params: Vec<i32>,
    intermediates: Vec<u8>,
    data: Vec<u8>,
    /// Tracks how many continuation bytes remain for a multi-byte UTF-8 char.
    utf8_remaining: u8,
    /// Accumulates the codepoint during UTF-8 decoding.
    utf8_codepoint: u32,
    /// Current numeric parameter being accumulated. `None` means no digit seen yet for this slot.
    current_param: Option<i32>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            params: Vec::with_capacity(MAX_PARAMS),
            intermediates: Vec::with_capacity(4),
            data: Vec::with_capacity(256),
            utf8_remaining: 0,
            utf8_codepoint: 0,
            current_param: None,
        }
    }

    /// Process a single byte through the state machine.
    pub fn advance<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        // Anywhere transitions: CAN (0x18) and SUB (0x1A) abort to ground.
        if byte == 0x18 || byte == 0x1A {
            self.state = State::Ground;
            handler.execute(byte);
            return;
        }

        // ESC while in any state except Ground transitions to Escape.
        // (In string states like OscString we handle ESC\ as ST instead.)
        if byte == 0x1B && !self.in_string_state() {
            self.clear();
            self.state = State::Escape;
            return;
        }

        match self.state {
            State::Ground => self.ground(handler, byte),
            State::Escape => self.escape(handler, byte),
            State::EscapeIntermediate => self.escape_intermediate(handler, byte),
            State::CsiEntry => self.csi_entry(handler, byte),
            State::CsiParam => self.csi_param(handler, byte),
            State::CsiIntermediate => self.csi_intermediate(handler, byte),
            State::DcsEntry => self.dcs_entry(handler, byte),
            State::DcsParam => self.dcs_param(handler, byte),
            State::DcsIntermediate => self.dcs_intermediate(handler, byte),
            State::DcsString => self.dcs_string(handler, byte),
            State::OscString => self.osc_string(handler, byte),
            State::SosString => self.sos_string(handler, byte),
            State::PmString => self.pm_string(handler, byte),
            State::ApcString => self.apc_string(handler, byte),
            State::Utf8 => self.utf8(handler, byte),
        }
    }

    /// Process a slice of bytes.
    pub fn process<H: Handler>(&mut self, handler: &mut H, data: &[u8]) {
        for &byte in data {
            self.advance(handler, byte);
        }
    }

    fn in_string_state(&self) -> bool {
        matches!(
            self.state,
            State::OscString
                | State::DcsString
                | State::SosString
                | State::PmString
                | State::ApcString
        )
    }

    fn clear(&mut self) {
        self.params.clear();
        self.intermediates.clear();
        self.data.clear();
        self.current_param = None;
    }

    /// Flush the current param accumulator into the params vec.
    fn finish_param(&mut self) {
        let val = self.current_param.unwrap_or(0);
        if self.params.len() < MAX_PARAMS {
            self.params.push(val);
        }
        self.current_param = None;
    }

    // ------- State handlers -------

    fn ground<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            // C0 controls
            0x00..=0x1A | 0x1C..=0x1F => handler.execute(byte),
            0x1B => {
                // Handled above but included for completeness; shouldn't reach here.
                self.clear();
                self.state = State::Escape;
            }
            // DEL - ignore
            0x7F => {}
            // Printable ASCII
            0x20..=0x7E => handler.print(byte as char),
            // UTF-8 lead bytes
            0xC0..=0xDF => {
                self.utf8_remaining = 1;
                self.utf8_codepoint = (byte as u32) & 0x1F;
                self.state = State::Utf8;
            }
            0xE0..=0xEF => {
                self.utf8_remaining = 2;
                self.utf8_codepoint = (byte as u32) & 0x0F;
                self.state = State::Utf8;
            }
            0xF0..=0xF7 => {
                self.utf8_remaining = 3;
                self.utf8_codepoint = (byte as u32) & 0x07;
                self.state = State::Utf8;
            }
            // C1 controls (8-bit): 0x80..=0x9F map to their 7-bit equivalents
            0x80..=0x8F | 0x91..=0x97 | 0x99 | 0x9A => handler.execute(byte),
            0x90 => {
                // DCS
                self.clear();
                self.state = State::DcsEntry;
            }
            0x98 => {
                // SOS
                self.clear();
                self.state = State::SosString;
            }
            0x9B => {
                // CSI
                self.clear();
                self.state = State::CsiEntry;
            }
            0x9C => {
                // ST - ignore in ground
            }
            0x9D => {
                // OSC
                self.clear();
                self.state = State::OscString;
            }
            0x9E => {
                // PM
                self.clear();
                self.state = State::PmString;
            }
            0x9F => {
                // APC
                self.clear();
                self.state = State::ApcString;
            }
            _ => {}
        }
    }

    fn utf8<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        // Expect continuation byte 10xxxxxx
        if byte & 0xC0 == 0x80 {
            self.utf8_codepoint = (self.utf8_codepoint << 6) | (byte as u32 & 0x3F);
            self.utf8_remaining -= 1;
            if self.utf8_remaining == 0 {
                if let Some(ch) = char::from_u32(self.utf8_codepoint) {
                    handler.print(ch);
                }
                self.state = State::Ground;
            }
        } else {
            // Invalid continuation; drop the sequence and reprocess this byte.
            self.state = State::Ground;
            self.advance(handler, byte);
        }
    }

    fn escape<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            // C0 controls pass through
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            // Intermediate bytes (space through /)
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::EscapeIntermediate;
            }
            // CSI
            b'[' => {
                self.clear();
                self.state = State::CsiEntry;
            }
            // OSC
            b']' => {
                self.data.clear();
                self.state = State::OscString;
            }
            // DCS
            b'P' => {
                self.params.clear();
                self.intermediates.clear();
                self.data.clear();
                self.current_param = None;
                self.state = State::DcsEntry;
            }
            // SOS
            b'X' => {
                self.data.clear();
                self.state = State::SosString;
            }
            // PM
            b'^' => {
                self.data.clear();
                self.state = State::PmString;
            }
            // APC
            b'_' => {
                self.data.clear();
                self.state = State::ApcString;
            }
            // Final bytes → esc_dispatch
            // Note: 0x5B='[', 0x5C='\', 0x5D=']', 0x5E='^', 0x5F='_' handled above
            0x30..=0x4F | 0x51..=0x57 | 0x59..=0x5A | 0x60..=0x7E => {
                handler.esc_dispatch(&self.intermediates, byte);
                self.state = State::Ground;
            }
            // ESC \ is ST in ground - just go back to ground
            b'\\' => {
                self.state = State::Ground;
            }
            // DEL - ignore
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn escape_intermediate<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            // More intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            // Final byte → esc_dispatch
            0x30..=0x7E => {
                handler.esc_dispatch(&self.intermediates, byte);
                self.state = State::Ground;
            }
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_entry<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            // Parameter bytes
            0x30..=0x39 => {
                // digit
                let digit = (byte - b'0') as i32;
                self.current_param = Some(self.current_param.unwrap_or(0) * 10 + digit);
                self.state = State::CsiParam;
            }
            b';' => {
                self.finish_param();
                self.state = State::CsiParam;
            }
            b':' => {
                // Subparam separator - treat like ; but mark with negative
                self.finish_param();
                self.state = State::CsiParam;
            }
            // Private-mode markers (< = > ?)
            0x3C..=0x3F => {
                self.intermediates.push(byte);
                self.state = State::CsiParam;
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            // Final byte → dispatch
            0x40..=0x7E => {
                self.finish_param();
                handler.csi_dispatch(&self.params, &self.intermediates, byte);
                self.state = State::Ground;
            }
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_param<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            // Digit
            0x30..=0x39 => {
                let digit = (byte - b'0') as i32;
                self.current_param = Some(self.current_param.unwrap_or(0) * 10 + digit);
            }
            b';' => {
                self.finish_param();
            }
            b':' => {
                self.finish_param();
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.finish_param();
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            // Private-mode markers appearing late - still collect
            0x3C..=0x3F => {
                // Technically invalid after params started, but some terminals send these.
                // Ignore to avoid getting stuck.
            }
            // Final byte → dispatch
            0x40..=0x7E => {
                self.finish_param();
                handler.csi_dispatch(&self.params, &self.intermediates, byte);
                self.state = State::Ground;
            }
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_intermediate<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            // Final byte
            0x40..=0x7E => {
                handler.csi_dispatch(&self.params, &self.intermediates, byte);
                self.state = State::Ground;
            }
            0x7F => {}
            _ => {
                // Invalid; discard sequence.
                self.state = State::Ground;
            }
        }
    }

    fn dcs_entry<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            0x30..=0x39 => {
                let digit = (byte - b'0') as i32;
                self.current_param = Some(self.current_param.unwrap_or(0) * 10 + digit);
                self.state = State::DcsParam;
            }
            b';' => {
                self.finish_param();
                self.state = State::DcsParam;
            }
            0x3C..=0x3F => {
                self.intermediates.push(byte);
                self.state = State::DcsParam;
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
            }
            // Immediately got a final byte: enter DcsString
            0x40..=0x7E => {
                self.finish_param();
                self.data.clear();
                self.state = State::DcsString;
            }
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn dcs_param<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            0x30..=0x39 => {
                let digit = (byte - b'0') as i32;
                self.current_param = Some(self.current_param.unwrap_or(0) * 10 + digit);
            }
            b';' => {
                self.finish_param();
            }
            b':' => {
                self.finish_param();
            }
            0x20..=0x2F => {
                self.finish_param();
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
            }
            0x40..=0x7E => {
                self.finish_param();
                self.data.clear();
                self.state = State::DcsString;
            }
            0x3C..=0x3F => {}
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn dcs_intermediate<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => handler.execute(byte),
            0x20..=0x2F => {
                self.intermediates.push(byte);
            }
            0x40..=0x7E => {
                self.finish_param();
                self.data.clear();
                self.state = State::DcsString;
            }
            0x7F => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn dcs_string<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            // ESC starts potential ST (ESC \)
            0x1B => {
                self.state = State::Escape;
                // We'll dispatch on seeing '\' in escape, but we need a way to know
                // we came from DCS. Instead, dispatch now since ESC exits the string.
                handler.dcs_dispatch(&self.params, &self.intermediates, &self.data);
            }
            // BEL can also terminate (some terminals accept this)
            0x07 => {
                handler.dcs_dispatch(&self.params, &self.intermediates, &self.data);
                self.state = State::Ground;
            }
            _ => {
                if self.data.len() < MAX_DATA {
                    self.data.push(byte);
                }
            }
        }
    }

    fn osc_string<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            // BEL terminates OSC
            0x07 => {
                handler.osc_dispatch(&self.data);
                self.state = State::Ground;
            }
            // ESC might be start of ST (ESC \)
            0x1B => {
                handler.osc_dispatch(&self.data);
                // Transition to Escape so ESC \ goes to Ground
                self.state = State::Escape;
            }
            _ => {
                if self.data.len() < MAX_DATA {
                    self.data.push(byte);
                }
            }
        }
    }

    fn sos_string<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x1B => {
                handler.sos_dispatch(&self.data);
                self.state = State::Escape;
            }
            0x07 => {
                handler.sos_dispatch(&self.data);
                self.state = State::Ground;
            }
            _ => {
                if self.data.len() < MAX_DATA {
                    self.data.push(byte);
                }
            }
        }
    }

    fn pm_string<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x1B => {
                handler.pm_dispatch(&self.data);
                self.state = State::Escape;
            }
            0x07 => {
                handler.pm_dispatch(&self.data);
                self.state = State::Ground;
            }
            _ => {
                if self.data.len() < MAX_DATA {
                    self.data.push(byte);
                }
            }
        }
    }

    fn apc_string<H: Handler>(&mut self, handler: &mut H, byte: u8) {
        match byte {
            0x1B => {
                handler.apc_dispatch(&self.data);
                self.state = State::Escape;
            }
            0x07 => {
                handler.apc_dispatch(&self.data);
                self.state = State::Ground;
            }
            _ => {
                if self.data.len() < MAX_DATA {
                    self.data.push(byte);
                }
            }
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test handler that records all dispatched events.
    #[derive(Default)]
    struct Recorder {
        prints: Vec<char>,
        executes: Vec<u8>,
        csi_calls: Vec<(Vec<i32>, Vec<u8>, u8)>,
        esc_calls: Vec<(Vec<u8>, u8)>,
        osc_calls: Vec<Vec<u8>>,
        dcs_calls: Vec<(Vec<i32>, Vec<u8>, Vec<u8>)>,
        apc_calls: Vec<Vec<u8>>,
        pm_calls: Vec<Vec<u8>>,
        sos_calls: Vec<Vec<u8>>,
    }

    impl Handler for Recorder {
        fn print(&mut self, ch: char) {
            self.prints.push(ch);
        }

        fn execute(&mut self, byte: u8) {
            self.executes.push(byte);
        }

        fn csi_dispatch(&mut self, params: &[i32], intermediates: &[u8], final_byte: u8) {
            self.csi_calls
                .push((params.to_vec(), intermediates.to_vec(), final_byte));
        }

        fn esc_dispatch(&mut self, intermediates: &[u8], final_byte: u8) {
            self.esc_calls.push((intermediates.to_vec(), final_byte));
        }

        fn osc_dispatch(&mut self, data: &[u8]) {
            self.osc_calls.push(data.to_vec());
        }

        fn dcs_dispatch(&mut self, params: &[i32], intermediates: &[u8], data: &[u8]) {
            self.dcs_calls
                .push((params.to_vec(), intermediates.to_vec(), data.to_vec()));
        }

        fn apc_dispatch(&mut self, data: &[u8]) {
            self.apc_calls.push(data.to_vec());
        }

        fn pm_dispatch(&mut self, data: &[u8]) {
            self.pm_calls.push(data.to_vec());
        }

        fn sos_dispatch(&mut self, data: &[u8]) {
            self.sos_calls.push(data.to_vec());
        }
    }

    fn parse(input: &[u8]) -> Recorder {
        let mut parser = Parser::new();
        let mut rec = Recorder::default();
        parser.process(&mut rec, input);
        rec
    }

    // --- Ground state ---

    #[test]
    fn test_printable_ascii() {
        let rec = parse(b"hello");
        assert_eq!(rec.prints, vec!['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_c0_controls() {
        let rec = parse(b"\x07\x08\n\r");
        assert_eq!(rec.executes, vec![0x07, 0x08, 0x0A, 0x0D]);
    }

    #[test]
    fn test_mixed_print_and_control() {
        let rec = parse(b"a\nb");
        assert_eq!(rec.prints, vec!['a', 'b']);
        assert_eq!(rec.executes, vec![0x0A]);
    }

    // --- CSI sequences ---

    #[test]
    fn test_csi_sgr_bold() {
        // ESC [ 1 m
        let rec = parse(b"\x1b[1m");
        assert_eq!(rec.csi_calls.len(), 1);
        let (params, intermediates, final_byte) = &rec.csi_calls[0];
        assert_eq!(params, &[1]);
        assert!(intermediates.is_empty());
        assert_eq!(*final_byte, b'm');
    }

    #[test]
    fn test_csi_sgr_reset() {
        // ESC [ 0 m
        let rec = parse(b"\x1b[0m");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![0]);
        assert_eq!(rec.csi_calls[0].2, b'm');
    }

    #[test]
    fn test_csi_sgr_no_params() {
        // ESC [ m  (implicit 0)
        let rec = parse(b"\x1b[m");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![0]);
        assert_eq!(rec.csi_calls[0].2, b'm');
    }

    #[test]
    fn test_csi_sgr_multiple_params() {
        // ESC [ 38;5;196 m  (256-color red foreground)
        let rec = parse(b"\x1b[38;5;196m");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![38, 5, 196]);
        assert_eq!(rec.csi_calls[0].2, b'm');
    }

    #[test]
    fn test_csi_cursor_up() {
        // ESC [ 5 A  (cursor up 5)
        let rec = parse(b"\x1b[5A");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![5]);
        assert_eq!(rec.csi_calls[0].2, b'A');
    }

    #[test]
    fn test_csi_cursor_position() {
        // ESC [ 10;20 H  (move to row 10, col 20)
        let rec = parse(b"\x1b[10;20H");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![10, 20]);
        assert_eq!(rec.csi_calls[0].2, b'H');
    }

    #[test]
    fn test_csi_erase_display() {
        // ESC [ 2 J  (erase entire display)
        let rec = parse(b"\x1b[2J");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![2]);
        assert_eq!(rec.csi_calls[0].2, b'J');
    }

    #[test]
    fn test_csi_private_mode() {
        // ESC [ ? 25 h  (show cursor - DECTCEM)
        let rec = parse(b"\x1b[?25h");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![25]);
        assert_eq!(rec.csi_calls[0].1, vec![b'?']);
        assert_eq!(rec.csi_calls[0].2, b'h');
    }

    #[test]
    fn test_csi_private_mode_reset() {
        // ESC [ ? 1049 l  (disable alt screen)
        let rec = parse(b"\x1b[?1049l");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![1049]);
        assert_eq!(rec.csi_calls[0].1, vec![b'?']);
        assert_eq!(rec.csi_calls[0].2, b'l');
    }

    #[test]
    fn test_multiple_csi_sequences() {
        // Bold, then red fg, then text, then reset
        let rec = parse(b"\x1b[1m\x1b[31mhello\x1b[0m");
        assert_eq!(rec.csi_calls.len(), 3);
        assert_eq!(rec.csi_calls[0].0, vec![1]); // bold
        assert_eq!(rec.csi_calls[1].0, vec![31]); // red
        assert_eq!(rec.csi_calls[2].0, vec![0]); // reset
        assert_eq!(rec.prints, vec!['h', 'e', 'l', 'l', 'o']);
    }

    // --- OSC sequences ---

    #[test]
    fn test_osc_title_bel() {
        // ESC ] 0;my title BEL
        let rec = parse(b"\x1b]0;my title\x07");
        assert_eq!(rec.osc_calls.len(), 1);
        assert_eq!(rec.osc_calls[0], b"0;my title");
    }

    #[test]
    fn test_osc_title_st() {
        // ESC ] 0;my title ESC backslash
        let rec = parse(b"\x1b]0;my title\x1b\\");
        assert_eq!(rec.osc_calls.len(), 1);
        assert_eq!(rec.osc_calls[0], b"0;my title");
    }

    #[test]
    fn test_osc_hyperlink() {
        // ESC ] 8;params;uri BEL
        let rec = parse(b"\x1b]8;;https://example.com\x07");
        assert_eq!(rec.osc_calls.len(), 1);
        assert_eq!(rec.osc_calls[0], b"8;;https://example.com");
    }

    // --- ESC sequences ---

    #[test]
    fn test_esc_ri() {
        // ESC M  (reverse index)
        let rec = parse(b"\x1bM");
        assert_eq!(rec.esc_calls.len(), 1);
        assert_eq!(rec.esc_calls[0].0, Vec::<u8>::new());
        assert_eq!(rec.esc_calls[0].1, b'M');
    }

    #[test]
    fn test_esc_decsc() {
        // ESC 7  (save cursor)
        let rec = parse(b"\x1b7");
        assert_eq!(rec.esc_calls.len(), 1);
        assert_eq!(rec.esc_calls[0].1, b'7');
    }

    #[test]
    fn test_esc_with_intermediate() {
        // ESC ( B  (designate US ASCII to G0)
        let rec = parse(b"\x1b(B");
        assert_eq!(rec.esc_calls.len(), 1);
        assert_eq!(rec.esc_calls[0].0, vec![b'(']);
        assert_eq!(rec.esc_calls[0].1, b'B');
    }

    // --- DCS sequences ---

    #[test]
    fn test_dcs_simple() {
        // ESC P 1 $ r ... ESC backslash
        let rec = parse(b"\x1bP1$rsome data\x1b\\");
        assert_eq!(rec.dcs_calls.len(), 1);
    }

    #[test]
    fn test_dcs_with_bel() {
        let rec = parse(b"\x1bPdata here\x07");
        assert_eq!(rec.dcs_calls.len(), 1);
    }

    // --- APC sequences ---

    #[test]
    fn test_apc_bel() {
        let rec = parse(b"\x1b_hello apc\x07");
        assert_eq!(rec.apc_calls.len(), 1);
        assert_eq!(rec.apc_calls[0], b"hello apc");
    }

    #[test]
    fn test_apc_st() {
        let rec = parse(b"\x1b_hello apc\x1b\\");
        assert_eq!(rec.apc_calls.len(), 1);
        assert_eq!(rec.apc_calls[0], b"hello apc");
    }

    // --- PM sequences ---

    #[test]
    fn test_pm_bel() {
        let rec = parse(b"\x1b^pm data\x07");
        assert_eq!(rec.pm_calls.len(), 1);
        assert_eq!(rec.pm_calls[0], b"pm data");
    }

    // --- SOS sequences ---

    #[test]
    fn test_sos_bel() {
        let rec = parse(b"\x1bXsos data\x07");
        assert_eq!(rec.sos_calls.len(), 1);
        assert_eq!(rec.sos_calls[0], b"sos data");
    }

    // --- UTF-8 ---

    #[test]
    fn test_utf8_2byte() {
        // é = 0xC3 0xA9
        let rec = parse("é".as_bytes());
        assert_eq!(rec.prints, vec!['é']);
    }

    #[test]
    fn test_utf8_3byte() {
        // → = 0xE2 0x86 0x92
        let rec = parse("→".as_bytes());
        assert_eq!(rec.prints, vec!['→']);
    }

    #[test]
    fn test_utf8_4byte() {
        // 🦀 = 0xF0 0x9F 0xA6 0x80
        let rec = parse("🦀".as_bytes());
        assert_eq!(rec.prints, vec!['🦀']);
    }

    #[test]
    fn test_utf8_mixed_with_ansi() {
        let rec = parse("héllo \x1b[1mwörld\x1b[0m".as_bytes());
        assert_eq!(
            rec.prints,
            vec!['h', 'é', 'l', 'l', 'o', ' ', 'w', 'ö', 'r', 'l', 'd']
        );
        assert_eq!(rec.csi_calls.len(), 2);
    }

    // --- Edge cases ---

    #[test]
    fn test_can_aborts_sequence() {
        // CAN (0x18) should abort a CSI sequence
        let rec = parse(b"\x1b[1\x18m");
        // The CAN causes execute(0x18), then 'm' is just a print
        assert!(rec.csi_calls.is_empty());
        assert_eq!(rec.executes, vec![0x18]);
        assert_eq!(rec.prints, vec!['m']);
    }

    #[test]
    fn test_empty_input() {
        let rec = parse(b"");
        assert!(rec.prints.is_empty());
        assert!(rec.executes.is_empty());
        assert!(rec.csi_calls.is_empty());
    }

    #[test]
    fn test_del_ignored() {
        let rec = parse(b"\x7F");
        assert!(rec.prints.is_empty());
        assert!(rec.executes.is_empty());
    }

    #[test]
    fn test_sgr_rgb_foreground() {
        // ESC [ 38;2;255;128;0 m
        let rec = parse(b"\x1b[38;2;255;128;0m");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![38, 2, 255, 128, 0]);
        assert_eq!(rec.csi_calls[0].2, b'm');
    }

    #[test]
    fn test_scroll_region() {
        // ESC [ 1;24 r  (set scroll region)
        let rec = parse(b"\x1b[1;24r");
        assert_eq!(rec.csi_calls.len(), 1);
        assert_eq!(rec.csi_calls[0].0, vec![1, 24]);
        assert_eq!(rec.csi_calls[0].2, b'r');
    }

    // --- Handler default impls compile (no-op) ---

    struct EmptyHandler;
    impl Handler for EmptyHandler {}

    #[test]
    fn test_default_handler_compiles() {
        let mut parser = Parser::new();
        let mut handler = EmptyHandler;
        parser.process(&mut handler, b"\x1b[1mhello\x1b[0m");
        // Just verify it doesn't panic.
    }
}
