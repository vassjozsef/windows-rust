windows-rust
============

Experiments with windows crate. Make sure to update rust compiler (`rustup update stable`).

The thread in `main.rs` is not needed, it is used to check which thread the message loop is running.

Message loop is absolutely required to receive frames in frames arrived callback. It must be on the same thread as capturer.

Although it seems that frame arrived callbacks are coming on the same thread as the handler is registered, we need to use `Arc`.

The trickiest part is how frames are passed from the frames arrived handler to the capturer (`Arc<Mutex<T>>`) and then to the main thread (`mpsc::channel`).
