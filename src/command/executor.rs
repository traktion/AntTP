use log::debug;
use tokio::sync::mpsc::{channel, Sender};
use crate::command::Command;

pub struct Executor {}

impl Executor {
    pub async fn start(buffer_size: usize) -> Sender<Box<dyn Command>> {
        let (command_executor, mut command_receiver) = channel::<Box<dyn Command>>(buffer_size);

        tokio::spawn(async move {
            while let Some(command) = command_receiver.recv().await {
                debug!("executor capacity: [{}]", command_receiver.capacity());
                command.execute().await.unwrap();
            }
        });

        command_executor
    }
}