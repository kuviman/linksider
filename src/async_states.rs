use super::*;

pub enum Event {
    Update(f64),
    Event(geng::Event),
    Draw,
}

pub struct Context {
    framebuffer: Rc<RefCell<ugli::Framebuffer<'static>>>,
    events: futures::channel::mpsc::Receiver<Event>,
}

impl Context {
    pub async fn wait(&mut self) -> Event {
        self.events.next().await.unwrap()
    }
    pub fn framebuffer(&self) -> RefMut<ugli::Framebuffer<'static>> {
        self.framebuffer.borrow_mut()
    }
}

pub fn as_state<Fut>(geng: &Geng, f: impl FnOnce(Context) -> Fut) -> impl geng::State
where
    Fut: Future + 'static,
{
    struct State<Fut: 'static> {
        geng: Geng,
        framebuffer: Rc<RefCell<ugli::Framebuffer<'static>>>,
        events: futures::channel::mpsc::Sender<Event>,
        future: Fut,
    }

    impl<Fut: future::FusedFuture + Unpin> State<Fut> {
        fn gen_event<'a>(&mut self, event: Event) {
            // TODO: is ignore good?
            let _ = self.events.try_send(event);
            if !self.future.is_terminated() {
                let _ = self.future.poll_unpin(&mut std::task::Context::from_waker(
                    futures::task::noop_waker_ref(),
                ));
            }
        }
    }

    impl<Fut: future::FusedFuture + Unpin> geng::State for State<Fut> {
        fn transition(&mut self) -> Option<geng::state::Transition> {
            self.future
                .is_terminated()
                .then_some(geng::state::Transition::Pop)
        }
        fn update(&mut self, delta_time: f64) {
            self.gen_event(Event::Update(delta_time));
        }
        fn handle_event(&mut self, event: geng::Event) {
            self.gen_event(Event::Event(event));
        }
        fn draw(&mut self, _actual_framebuffer: &mut ugli::Framebuffer) {
            // TODO this is wrong
            *self.framebuffer.borrow_mut() = ugli::Framebuffer::default(self.geng.ugli());
            self.gen_event(Event::Draw);
        }
    }

    let (sender, receiver) = futures::channel::mpsc::channel(0);

    // TODO framebuffer should be handled differently
    let framebuffer = Rc::new(RefCell::new(ugli::Framebuffer::default(geng.ugli())));
    let waiter = Context {
        framebuffer: framebuffer.clone(),
        events: receiver,
    };

    let future = f(waiter);

    State {
        geng: geng.clone(),
        framebuffer,
        events: sender,
        future: future.boxed_local().fuse(),
    }
}
