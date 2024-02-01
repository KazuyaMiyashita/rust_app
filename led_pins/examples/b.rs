use std::marker::PhantomData;

trait Scheduler<F>
where
    F: Fn() -> (),
{
    fn schedule_at(&mut self, at: u32);
    fn set_callback(&mut self, f: F);
}

struct MockScheduler<F>
where
    F: Fn() -> (),
{
    counter: u32,
    maybe_next_schedule: Option<u32>,
    maybe_callback: Option<Box<F>>,
}
impl<F> MockScheduler<F>
where
    F: Fn() -> (),
{
    fn new() -> Self {
        MockScheduler {
            counter: 0,
            maybe_next_schedule: None,
            maybe_callback: None,
        }
    }
    fn next(&mut self) {
        match self.maybe_next_schedule {
            Some(schedule) if schedule == self.counter => {
                if let Some(callback) = &self.maybe_callback {
                    callback()
                }
                self.maybe_next_schedule = None;
            }
            _ => (),
        }
        self.counter += 1;
    }
}
impl<F> Scheduler<F> for MockScheduler<F>
where
    F: Fn() -> (),
{
    fn schedule_at(&mut self, at: u32) {
        self.maybe_next_schedule = Some(at);
    }
    fn set_callback(&mut self, f: F) {
        self.maybe_callback = Some(Box::new(f))
    }
}

struct App<'a, F, S>
where
    F: Fn() -> (),
    S: Scheduler<F>,
{
    scheduler: &'a mut S,
}
impl<'a, F, S> App<'a, F, S>
where
    F: Fn() -> (),
    S: Scheduler<F>,
{
    pub fn new(scheduler: &mut S) -> Self {
        App { scheduler }
    }

    pub fn hello(&self) {
        println!("hello!");
    }

    pub fn schedule_at(&mut self, at: u32) {
        self.scheduler.schedule_at(at);
    }
}

fn main() {
    let mut scheduler = MockScheduler::new();
    let mut app = App::new(&mut scheduler);
    app.hello();
    app.scheduler.set_callback(|| app.hello());
    //                          ^^^^^^^^^^^^^^ cyclic type of infinite size
    // app.scheduler.set_callback(&|| println!(""));

    app.scheduler.schedule_at(2);
    for _ in 0..=3 {
        println!("counter: {}", scheduler.counter);
        scheduler.next();
    }
    // hello!
    // counter: 0
    // counter: 1
    // counter: 2
    // a
    // counter: 3
}
