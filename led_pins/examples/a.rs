trait Scheduler {
    fn schedule_at(&mut self, at: u32);
}

struct App<S: Scheduler> {
    scheduler: S,
}
impl<S> App<S>
where
    S: Scheduler,
{
    pub fn new(scheduler: S) -> Self {
        App { scheduler }
    }
    pub fn set_schedule_at(&self, at: u32) {
        self.scheduler.schedule_at(at);
    }
    pub fn interrupt_handler(&mut self) {
        println!("interrut_handler called.")
    }
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
                if let Some(callback) = self.maybe_callback {
                    callback()
                }
                self.maybe_next_schedule = None;
            }
            _ => (),
        }
        self.counter += 1;
    }
    fn set_callback(&mut self, f: F) {
        self.maybe_callback = Some(Box::new(f))
    }
}
impl<F> Scheduler for MockScheduler<F>
where
    F: Fn() -> (),
{
    fn schedule_at(&mut self, at: u32) {
        self.maybe_next_schedule = Some(at);
    }
}

fn main() {
    let scheduler = MockScheduler::new();
    let app = App::new(scheduler);
    scheduler.set_callback(|| app.interrupt_handler());
    //                     ^^^^^^^^^^^^^^^^^^^^^^^^^^ cyclic type of infinite size
    app.set_schedule_at(2);
    for _ in 0..=3 {
        scheduler.next();
    }
}
