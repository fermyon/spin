use {
    crate::wasi::poll::poll2 as poll,
    anyhow::Result,
    std::{
        cell::RefCell,
        collections::HashMap,
        future::Future,
        mem,
        pin::Pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    },
};

#[derive(Clone, Default)]
pub struct Wakers(Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>);

impl Wakers {
    pub fn insert(&self, pollable: poll::Pollable, waker: Waker) {
        self.0.borrow_mut().entry(pollable).or_default().push(waker);
    }

    pub fn run(&self, mut future: Pin<&mut impl Future<Output = Result<()>>>) -> Result<()> {
        struct DummyWaker;

        impl Wake for DummyWaker {
            fn wake(self: Arc<Self>) {}
        }

        let waker = Arc::new(DummyWaker).into();

        loop {
            match future.as_mut().poll(&mut Context::from_waker(&waker)) {
                Poll::Pending => {
                    assert!(!self.0.borrow().is_empty());

                    let mut new_wakers = HashMap::new();

                    let (pollables, wakers) = mem::take::<HashMap<_, _>>(&mut self.0.borrow_mut())
                        .into_iter()
                        .unzip::<_, _, Vec<_>, Vec<_>>();

                    for ((ready, pollable), wakers) in poll::poll_oneoff(&pollables)
                        .into_iter()
                        .zip(&pollables)
                        .zip(wakers)
                    {
                        if ready {
                            for waker in wakers {
                                waker.wake();
                            }
                        } else {
                            new_wakers.insert(*pollable, wakers);
                        }
                    }

                    *self.0.borrow_mut() = new_wakers;
                }
                Poll::Ready(result) => break result,
            }
        }
    }
}
