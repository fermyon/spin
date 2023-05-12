use once_cell::sync::OnceCell;
use termcolor::{ColorSpec, StandardStream, StandardStreamLock, WriteColor};
static COLOR_ERR: OnceCell<StandardStream> = OnceCell::new();

/// A wrapper around a standard stream lock that resets the color on drop
pub(crate) struct ColorText(StandardStreamLock<'static>);

impl ColorText {
    pub(crate) fn stderr(spec: ColorSpec) -> ColorText {
        let err = COLOR_ERR.get_or_init(|| {
            let choice = if atty::is(atty::Stream::Stderr) {
                termcolor::ColorChoice::Auto
            } else {
                termcolor::ColorChoice::Never
            };
            StandardStream::stderr(choice)
        });
        set_color(err, spec)
    }
}

impl std::io::Write for ColorText {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl WriteColor for ColorText {
    fn supports_color(&self) -> bool {
        self.0.supports_color()
    }

    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        self.0.set_color(spec)
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.0.reset()
    }
}

impl Drop for ColorText {
    fn drop(&mut self) {
        let _ = self.reset();
    }
}

fn set_color(out: &'static StandardStream, spec: ColorSpec) -> ColorText {
    let mut lock = out.lock();
    let _ = lock.set_color(&spec);
    ColorText(lock)
}

macro_rules! error {
    ($($arg:tt)*) => {
        $crate::colors::ceprint!($crate::colors::colors::red(), "Error");
        eprint!(": ");
        eprintln!($($arg)*);
    };
}

macro_rules! ceprint {
    ($color:expr, $($arg:tt)*) => {
        use std::io::Write;
        let mut out = $crate::colors::ColorText::stderr($color);
        let _ = write!(out, $($arg)*);
        drop(out); // Reset colors
    };
}

pub(crate) use ceprint;
pub(crate) use error;

pub(crate) mod colors {
    use termcolor::{Color, ColorSpec};

    pub fn red() -> ColorSpec {
        new(Color::Red, true)
    }

    fn new(color: Color, bold: bool) -> ColorSpec {
        let mut s = ColorSpec::new();
        s.set_fg(Some(color)).set_bold(bold);
        s
    }
}
