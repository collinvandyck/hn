use rusqlite::Connection;
use tokio::sync::mpsc;

use super::StorageCommand;
pub use super::migrations::run_migrations;
use super::queries;

pub fn run_worker(conn: Connection, mut cmd_rx: mpsc::Receiver<StorageCommand>) {
    while let Some(cmd) = cmd_rx.blocking_recv() {
        match cmd {
            StorageCommand::SaveStory { story, reply } => {
                let result = queries::save_story(&conn, &story);
                let _ = reply.send(result);
            }
            StorageCommand::GetStory { id, reply } => {
                let result = queries::get_story(&conn, id);
                let _ = reply.send(result);
            }
            StorageCommand::SaveComments {
                story_id,
                comments,
                reply,
            } => {
                let result = queries::save_comments(&conn, story_id, &comments);
                let _ = reply.send(result);
            }
            StorageCommand::GetComments { story_id, reply } => {
                let result = queries::get_comments(&conn, story_id);
                let _ = reply.send(result);
            }
            StorageCommand::SaveFeed { feed, ids, reply } => {
                let result = queries::save_feed(&conn, feed, &ids);
                let _ = reply.send(result);
            }
            StorageCommand::GetFeed { feed, reply } => {
                let result = queries::get_feed(&conn, feed);
                let _ = reply.send(result);
            }
            StorageCommand::MarkStoryRead { id, reply } => {
                let result = queries::mark_story_read(&conn, id);
                let _ = reply.send(result);
            }
        }
    }
}
