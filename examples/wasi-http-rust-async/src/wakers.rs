use {
    crate::wasi::io::poll,
    anyhow::Result,
    std::{
        cell::RefCell,
        future::Future,
        mem,
        pin::Pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    },
};

#[derive(Clone, Default)]
pub struct Wakers(Rc<RefCell<Vec<(poll::Pollable, Waker)>>>);

impl Wakers {
    pub fn insert(&self, pollable: poll::Pollable, waker: Waker) {
        self.0.borrow_mut().push((pollable, waker));
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

                    let mut new_wakers = Vec::new();

                    let wakers = mem::take::<Vec<_>>(&mut self.0.borrow_mut());

                    let pollables = wakers
                        .iter()
                        .map(|(pollable, _)| pollable)
                        .collect::<Vec<_>>();

                    let mut ready = vec![false; wakers.len()];

                    for index in poll::poll_list(&pollables) {
                        ready[usize::try_from(index).unwrap()] = true;
                    }

                    for (ready, (pollable, waker)) in ready.into_iter().zip(wakers) {
                        if ready {
                            waker.wake()
                        } else {
                            new_wakers.push((pollable, waker));
                        }
                    }

                    *self.0.borrow_mut() = new_wakers;
                }
                Poll::Ready(result) => break result,
            }
        }
    }
}
