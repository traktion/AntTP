use std::time::Duration;
use actix_web::web::Data;
use indexmap::IndexMap;
use log::{debug, error, warn};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use crate::client::command::Command;
use crate::client::command::command_details::{CommandDetails, CommandState};
use crate::client::command::command_details::CommandState::{ABORTED, COMPLETED, RUNNING};
use crate::client::command::error::CommandError;

pub struct Executor {}

impl Executor {
    pub async fn start(buffer_size: usize, executor_map: Data<Mutex<IndexMap<u128, CommandDetails>>>) -> Sender<Box<dyn Command>> {
        let (command_queue_sender, mut command_queue_receiver) = channel::<Box<dyn Command>>(buffer_size);
        let (command_executor_sender, mut command_executor_receiver) = channel::<Box<dyn Command>>(buffer_size);

        let pre_executor_map = executor_map.clone();

        // read the queue and insert command details into the executor map
        tokio::spawn(async move {
            while let Some(command) = command_queue_receiver.recv().await {
                let command_details = CommandDetails::new(&command);
                debug!("command buffered: [{:?}]", command_details);
                pre_executor_map.get_ref().lock().await.insert(command.id(), command_details);

                command_executor_sender.send(command).await.unwrap();
            }
        });

        // execute commands and update command details in the executor map
        tokio::spawn(async move {
            let mut last_hash = vec![];
            while let Some(command) = command_executor_receiver.recv().await {
                let command_action_hash = command.action_hash();
                if last_hash == command_action_hash {
                    Self::update_executor_map(&executor_map, buffer_size, command.id(), ABORTED).await;
                } else {
                    Self::update_executor_map(&executor_map, buffer_size, command.id(), RUNNING).await;

                    let mut attempt = 1;
                    loop {
                        match command.execute().await {
                            Ok(_) => break,
                            Err(error) => {
                                match error {
                                    CommandError::Unrecoverable(_) => {
                                        error!("failed to execute command [{}] with single attempt (skipping): [{:?}]", command.id(), error);
                                        break;
                                    },
                                    CommandError::Recoverable(_) => {
                                        if attempt <= 5 {
                                            warn!("failed to execute command [{}] on attempt [{}] (retrying): [{:?}]", command.id(), attempt, error);
                                            let backoff =  attempt * attempt;
                                            sleep(Duration::from_secs(backoff)).await;
                                            attempt += 1;
                                        } else {
                                            error!("failed to execute command [{}] after attempt [{}] (skipping): [{:?}]", command.id(), attempt, error);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Self::update_executor_map(&executor_map, buffer_size, command.id(), COMPLETED).await;
                    last_hash = command_action_hash;
                }
            }
        });

        command_queue_sender
    }

    /*async fn log_executor_map(executor_map: &Data<Mutex<IndexMap<u128, CommandDetails>>>) {
        let mut executor_map_string = String::new();
        executor_map.lock().await.iter().for_each(|(_, v)| executor_map_string += &format!("{:?},", v).as_str());
        debug!("command queue {:?}", executor_map_string);
    }*/

    async fn update_executor_map(executor_map: &Data<Mutex<IndexMap<u128, CommandDetails>>>, buffer_size: usize, command_id: u128, command_state: CommandState) {
        let maybe_command_details = match executor_map.get_ref().lock().await.get(&command_id) {
            Some(command_details) => {
                let mut new_command_details = command_details.clone();
                new_command_details.set_state(command_state);
                Some(new_command_details)
            },
            None => None, // should never happen
        };
        if let Some(command_detail) = maybe_command_details {
            executor_map.get_ref().lock().await.insert(command_id.clone(), command_detail.clone());
            debug!("command status: [{:?}]", command_detail);
        }
        if executor_map.get_ref().lock().await.len() > (buffer_size * 128) {
            // todo: tune size/content to prevent useful records scrolling out of the map
            // todo: improve performance as this is O(n)
            executor_map.get_ref().lock().await.shift_remove_index(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use async_trait::async_trait;
    use crate::client::command::Command;
    use crate::client::command::error::CommandError;

    #[derive(Clone)]
    struct MockCommand {
        id: u128,
        action_hash: Vec<u8>,
        results: Arc<Mutex<Vec<Result<(), CommandError>>>>,
    }

    impl MockCommand {
        fn new(id: u128, action_hash: Vec<u8>, results: Vec<Result<(), CommandError>>) -> Self {
            Self {
                id,
                action_hash,
                results: Arc::new(Mutex::new(results)),
            }
        }
    }

    #[async_trait]
    impl Command for MockCommand {
        async fn execute(&self) -> Result<(), CommandError> {
            let mut results = self.results.lock().await;
            if let Some(res) = results.pop() {
                res
            } else {
                Ok(())
            }
        }

        fn action_hash(&self) -> Vec<u8> {
            self.action_hash.clone()
        }

        fn id(&self) -> u128 {
            self.id
        }
    }

    async fn wait_for_state(executor_map: &Data<Mutex<IndexMap<u128, CommandDetails>>>, command_id: u128, expected_state: CommandState) {
        for _ in 0..100 { // Wait up to 10 seconds (100 * 100ms)
            let map = executor_map.lock().await;
            if let Some(details) = map.get(&command_id) {
                if details.state() == &expected_state {
                    return;
                }
            }
            drop(map);
            sleep(Duration::from_millis(100)).await;
        }
        panic!("Timed out waiting for command {} to reach state {:?}", command_id, expected_state);
    }

    #[tokio::test]
    async fn test_execute_success() {
        let executor_map = Data::new(Mutex::new(IndexMap::new()));
        let sender = Executor::start(10, executor_map.clone()).await;

        let command = MockCommand::new(1, vec![1], vec![Ok(())]);
        sender.send(Box::new(command)).await.unwrap();

        wait_for_state(&executor_map, 1, COMPLETED).await;
    }

    #[tokio::test]
    async fn test_execute_failure_recoverable() {
        let executor_map = Data::new(Mutex::new(IndexMap::new()));
        let sender = Executor::start(10, executor_map.clone()).await;

        // Fail twice with recoverable error, then succeed
        // Note: results are popped, so push in reverse order: Ok, Err, Err
        let results = vec![
            Ok(()),
            Err(CommandError::Recoverable("fail 2".to_string())),
            Err(CommandError::Recoverable("fail 1".to_string())),
        ];
        let command = MockCommand::new(2, vec![2], results);
        sender.send(Box::new(command)).await.unwrap();

        wait_for_state(&executor_map, 2, COMPLETED).await;
    }

    #[tokio::test]
    async fn test_execute_failure_unrecoverable() {
        let executor_map = Data::new(Mutex::new(IndexMap::new()));
        let sender = Executor::start(10, executor_map.clone()).await;

        let results = vec![
            Err(CommandError::Unrecoverable("fail".to_string())),
        ];
        let command = MockCommand::new(3, vec![3], results);
        sender.send(Box::new(command)).await.unwrap();

        // Even unrecoverable errors result in COMPLETED state in current implementation
        wait_for_state(&executor_map, 3, COMPLETED).await;
    }

    #[tokio::test]
    async fn test_execute_duplicate() {
        let executor_map = Data::new(Mutex::new(IndexMap::new()));
        let sender = Executor::start(10, executor_map.clone()).await;

        let command1 = MockCommand::new(4, vec![4], vec![Ok(())]);
        let command2 = MockCommand::new(5, vec![4], vec![Ok(())]); // Same hash as command1

        sender.send(Box::new(command1)).await.unwrap();
        // Wait for first to complete
        wait_for_state(&executor_map, 4, COMPLETED).await;

        sender.send(Box::new(command2)).await.unwrap();
        // Second should be aborted because hash matches last executed
        wait_for_state(&executor_map, 5, ABORTED).await;
    }
}