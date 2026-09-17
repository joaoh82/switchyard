//! Rebuilding a terminal's screen for a viewer that attaches late.
//!
//! Replaying raw bytes is not enough: a full-screen program may have entered the alternate screen
//! megabytes ago. Instead the host keeps a headless terminal and serialises its *state* — a byte
//! sequence that, written to a fresh emulator of the same size, reproduces scrollback, screen,
//! cursor and input modes.

/// Serialise `parser`'s scrollback and screen. Leaves the parser's scrollback view at the bottom.
pub(crate) fn render(parser: &mut vt100::Parser) -> Vec<u8> {
    let (rows, cols) = parser.screen().size();
    let mut out = Vec::new();

    // Start from a known state, whatever the viewer showed before.
    out.extend_from_slice(b"\x1bc");

    // Scrollback exists only on the primary screen.
    if !parser.screen().alternate_screen() {
        let history = scrollback_rows(parser, rows, cols);
        if !history.is_empty() {
            for row in &history {
                out.extend_from_slice(row);
                out.extend_from_slice(b"\x1b[m\r\n");
            }
            // Push every history line off the visible area and into the viewer's own
            // scrollback, because the screen state below begins by clearing the display.
            out.extend(std::iter::repeat_n(b'\n', usize::from(rows)));
        }
    }

    if parser.screen().alternate_screen() {
        out.extend_from_slice(b"\x1b[?1049h");
    }
    out.extend_from_slice(&parser.screen().state_formatted());
    out
}

/// Every scrollback line, oldest first, each as formatted bytes without a line terminator.
fn scrollback_rows(parser: &mut vt100::Parser, rows: u16, cols: u16) -> Vec<Vec<u8>> {
    let page = usize::from(rows);
    // `set_scrollback` clamps to what is available, which is how we learn the length.
    parser.screen_mut().set_scrollback(usize::MAX);
    let total = parser.screen().scrollback();

    let mut history = Vec::with_capacity(total);
    let mut offset = total;
    while offset > 0 {
        // With the view scrolled back by `offset`, its first `min(offset, page)` rows are history.
        parser.screen_mut().set_scrollback(offset);
        let take = offset.min(page);
        history.extend(parser.screen().rows_formatted(0, cols).take(take));
        offset -= take;
    }
    parser.screen_mut().set_scrollback(0);
    history
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed `snapshot` to a fresh terminal and return it.
    fn replay(snapshot: &[u8], rows: u16, cols: u16) -> vt100::Parser {
        let mut viewer = vt100::Parser::new(rows, cols, 1000);
        viewer.process(snapshot);
        viewer
    }

    #[test]
    fn reproduces_screen_cursor_and_colours() {
        let mut source = vt100::Parser::new(5, 20, 1000);
        source.process(b"plain \x1b[31mred\x1b[m\r\nsecond line\x1b[2;4H");

        let viewer = replay(&render(&mut source), 5, 20);

        assert_eq!(viewer.screen().contents(), source.screen().contents());
        assert_eq!(viewer.screen().cursor_position(), (1, 3));
        let cell = viewer.screen().cell(0, 6).unwrap();
        assert_eq!(cell.contents(), "r");
        assert_eq!(cell.fgcolor(), vt100::Color::Idx(1));
    }

    #[test]
    fn carries_scrollback_in_order() {
        let mut source = vt100::Parser::new(4, 20, 1000);
        for n in 1..=25 {
            source.process(format!("line {n}\r\n").as_bytes());
        }

        let mut viewer = replay(&render(&mut source), 4, 20);

        assert_eq!(viewer.screen().contents(), source.screen().contents());
        let theirs = scrollback_text(&mut viewer);
        let ours = scrollback_text(&mut source);
        assert_eq!(ours.first().map(String::as_str), Some("line 1"));
        // The viewer holds our history, in order, followed only by padding it scrolled away.
        assert_eq!(&theirs[..ours.len()], &ours[..]);
        assert!(theirs[ours.len()..].iter().all(String::is_empty));
    }

    #[test]
    fn restores_the_alternate_screen_and_input_modes() {
        let mut source = vt100::Parser::new(5, 20, 1000);
        source.process(b"shell prompt\r\n\x1b[?1049h\x1b[?2004h\x1b[?25lfull-screen app");

        let viewer = replay(&render(&mut source), 5, 20);

        assert!(viewer.screen().alternate_screen());
        assert!(viewer.screen().bracketed_paste());
        assert!(viewer.screen().hide_cursor());
        assert_eq!(viewer.screen().contents(), "full-screen app");
    }

    #[test]
    fn leaves_the_source_scrolled_to_the_bottom() {
        let mut source = vt100::Parser::new(3, 10, 100);
        for n in 1..=10 {
            source.process(format!("{n}\r\n").as_bytes());
        }
        let before = source.screen().contents();
        render(&mut source);
        assert_eq!(source.screen().scrollback(), 0);
        assert_eq!(source.screen().contents(), before);
    }

    fn scrollback_text(parser: &mut vt100::Parser) -> Vec<String> {
        let (rows, cols) = parser.screen().size();
        scrollback_rows(parser, rows, cols)
            .into_iter()
            .map(|row| {
                let mut p = vt100::Parser::new(1, cols, 0);
                p.process(&row);
                p.screen().contents()
            })
            .collect()
    }
}
