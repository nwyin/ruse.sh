use ruse_runtime::Cmd;
use ruse_runtime::Msg;
use ruse_runtime::pane::Pane;

use crate::cursor::Cursor;
use crate::filepicker::FilePicker;
use crate::list::List;
use crate::paginator::Paginator;
use crate::progress::Progress;
use crate::spinner::Spinner;
use crate::stopwatch::Stopwatch;
use crate::table::Table;
use crate::textarea::TextArea;
use crate::textinput::TextInput;
use crate::timer::Timer;
use crate::viewport::Viewport;

impl Pane for TextInput {
    fn update(&mut self, msg: &Msg) -> Cmd {
        TextInput::update(self, msg)
    }
    fn view(&self) -> String {
        TextInput::view(self)
    }
    fn focus(&mut self) -> Cmd {
        TextInput::focus(self)
    }
    fn blur(&mut self) {
        TextInput::blur(self);
    }
    fn focused(&self) -> bool {
        TextInput::focused(self)
    }
}

impl Pane for TextArea {
    fn update(&mut self, msg: &Msg) -> Cmd {
        TextArea::update(self, msg)
    }
    fn view(&self) -> String {
        TextArea::view(self)
    }
    fn focus(&mut self) -> Cmd {
        TextArea::focus(self)
    }
    fn blur(&mut self) {
        TextArea::blur(self);
    }
    fn focused(&self) -> bool {
        TextArea::focused(self)
    }
}

impl Pane for Viewport {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Viewport::update(self, msg)
    }
    fn view(&self) -> String {
        Viewport::view(self)
    }
}

impl Pane for List {
    fn update(&mut self, msg: &Msg) -> Cmd {
        List::update(self, msg)
    }
    fn view(&self) -> String {
        List::view(self)
    }
}

impl Pane for Table {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Table::update(self, msg)
    }
    fn view(&self) -> String {
        Table::view(self)
    }
    fn focus(&mut self) -> Cmd {
        Table::focus(self);
        None
    }
    fn blur(&mut self) {
        Table::blur(self);
    }
}

impl Pane for Spinner {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Spinner::update(self, msg)
    }
    fn view(&self) -> String {
        Spinner::view(self)
    }
    fn init(&mut self) -> Cmd {
        Spinner::init(self)
    }
}

impl Pane for Progress {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Progress::update(self, msg)
    }
    fn view(&self) -> String {
        Progress::view(self)
    }
}

impl Pane for Paginator {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Paginator::update(self, msg)
    }
    fn view(&self) -> String {
        Paginator::view(self)
    }
}

impl Pane for Stopwatch {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Stopwatch::update(self, msg)
    }
    fn view(&self) -> String {
        Stopwatch::view(self)
    }
}

impl Pane for Timer {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Timer::update(self, msg)
    }
    fn view(&self) -> String {
        Timer::view(self)
    }
}

impl Pane for FilePicker {
    fn update(&mut self, msg: &Msg) -> Cmd {
        FilePicker::update(self, msg)
    }
    fn view(&self) -> String {
        FilePicker::view(self)
    }
    fn init(&mut self) -> Cmd {
        FilePicker::init(self)
    }
}

impl Pane for Cursor {
    fn update(&mut self, msg: &Msg) -> Cmd {
        Cursor::update(self, msg)
    }
    fn view(&self) -> String {
        Cursor::view(self)
    }
    fn focus(&mut self) -> Cmd {
        Cursor::focus(self)
    }
    fn blur(&mut self) {
        Cursor::blur(self);
    }
    fn focused(&self) -> bool {
        Cursor::focused(self)
    }
}
