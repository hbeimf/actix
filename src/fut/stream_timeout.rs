use std::time::Duration;
use futures::{Async, Future, Poll};
use tokio_core::reactor::Timeout as TokioTimeout;

use fut::ActorStream;
use actor::Actor;
use arbiter::Arbiter;


/// Future for the `timeout` combinator, interrupts computations if it takes more
/// than `timeout`.
///
/// This is created by the `ActorFuture::timeout()` method.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct StreamTimeout<S> where S: ActorStream
{
    stream: S,
    err: S::Error,
    dur: Duration,
    timeout: Option<TokioTimeout>,
}

pub fn new<S>(stream: S, timeout: Duration, err: S::Error) -> StreamTimeout<S>
    where S: ActorStream, S::Error: Clone
{
    StreamTimeout {stream, err, dur: timeout, timeout: None}
}

impl<S> ActorStream for StreamTimeout<S> where S: ActorStream, S::Error: Clone
{
    type Item = S::Item;
    type Error = S::Error;
    type Actor = S::Actor;

    fn poll(&mut self,
            act: &mut S::Actor,
            ctx: &mut <S::Actor as Actor>::Context) -> Poll<Option<S::Item>, S::Error>
    {
        match self.stream.poll(act, ctx) {
            Ok(Async::Ready(res)) => {
                self.timeout.take();
                return Ok(Async::Ready(res))
            }
            Ok(Async::NotReady) => (),
            Err(err) => return Err(err)
        }

        if self.timeout.is_none() {
            self.timeout = Some(
                TokioTimeout::new(self.dur, Arbiter::handle()).unwrap());
        }

        // check timeout
        match self.timeout.as_mut().unwrap().poll() {
            Ok(Async::Ready(())) => (),
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(_) => unreachable!(),
        }
        self.timeout.take();

        Err(self.err.clone())
    }
}
