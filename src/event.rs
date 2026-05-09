use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};
use std::time::{Duration, Instant};
use std::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    #[allow(dead_code)]
    Mouse(MouseEvent),
    #[allow(dead_code)]
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _tx: mpsc::Sender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(tick_rate);
        let tx2 = tx.clone();

        std::thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if crossterm::event::poll(timeout).unwrap() {
                    match event::read().unwrap() {
                        CrosstermEvent::Key(e) => {
                            if e.kind == KeyEventKind::Press {
                                if tx.send(Event::Key(e)).is_err() {
                                    return;
                                }
                            }
                        }
                        CrosstermEvent::Mouse(e) => {
                            if tx.send(Event::Mouse(e)).is_err() {
                                return;
                            }
                        }
                        CrosstermEvent::Resize(w, h) => {
                            if tx.send(Event::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if tx.send(Event::Tick).is_err() {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { rx, _tx: tx2 }
    }

    pub fn next(&mut self) -> anyhow::Result<Event> {
        Ok(self.rx.recv().unwrap_or(Event::Tick))
    }
}
