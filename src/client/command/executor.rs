use log::debug;
use tokio::sync::mpsc::{channel, Sender};
use crate::client::command::Command;

pub struct Executor {}

impl Executor {
    pub async fn start(buffer_size: usize) -> Sender<Box<dyn Command>> {
        let (command_executor, mut command_receiver) = channel::<Box<dyn Command>>(buffer_size);

        tokio::spawn(async move {
            let mut last_hash = vec![];
            while let Some(command) = command_receiver.recv().await {
                debug!("executor capacity: [{}]", command_receiver.capacity());
                if last_hash == command.get_hash() {
                    debug!("skipping duplicate command");
                    continue;
                } else {
                    // don't execute duplicate commands
                    command.execute().await.unwrap();
                    last_hash = command.get_hash();
                    debug!("executor completed for: [{}]", command_receiver.capacity());

                }
            }
        });

        command_executor
    }
}