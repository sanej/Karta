//! Task memory and persistence
//!
//! Stores task history so Karta can follow up on previous tasks
//! and maintain context across sessions.

use crate::error::{KartaError, Result};
use crate::task::Task;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Task memory - persists and retrieves tasks
pub struct TaskMemory {
    /// Base directory for task storage
    storage_dir: PathBuf,

    /// In-memory cache of tasks
    cache: HashMap<Uuid, Task>,
}

impl TaskMemory {
    /// Create a new task memory with the given storage directory
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_dir)?;

        let mut memory = TaskMemory {
            storage_dir,
            cache: HashMap::new(),
        };

        // Load existing tasks into cache
        memory.load_all()?;

        Ok(memory)
    }

    /// Create task memory in the default location
    pub fn default_location() -> Result<Self> {
        let storage_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("karta")
            .join("tasks");

        Self::new(storage_dir)
    }

    /// Save a task
    pub fn save(&mut self, task: &Task) -> Result<()> {
        let path = self.task_path(&task.id);
        let content = serde_json::to_string_pretty(task)?;
        std::fs::write(&path, content)?;

        self.cache.insert(task.id, task.clone());

        Ok(())
    }

    /// Get a task by ID
    pub fn get(&self, id: &Uuid) -> Option<&Task> {
        self.cache.get(id)
    }

    /// Get a mutable task by ID
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Task> {
        self.cache.get_mut(id)
    }

    /// List all tasks
    pub fn list(&self) -> Vec<&Task> {
        self.cache.values().collect()
    }

    /// List tasks by state
    pub fn list_by_state(&self, state: &crate::task::TaskState) -> Vec<&Task> {
        self.cache
            .values()
            .filter(|t| &t.state == state)
            .collect()
    }

    /// List active tasks
    pub fn list_active(&self) -> Vec<&Task> {
        self.cache.values().filter(|t| t.is_active()).collect()
    }

    /// List completed tasks
    pub fn list_completed(&self) -> Vec<&Task> {
        self.cache.values().filter(|t| t.is_complete()).collect()
    }

    /// Search tasks by target name
    pub fn search_by_target(&self, query: &str) -> Vec<&Task> {
        let query_lower = query.to_lowercase();
        self.cache
            .values()
            .filter(|t| t.target.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get recent tasks (last N)
    pub fn recent(&self, limit: usize) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.cache.values().collect();
        tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        tasks.into_iter().take(limit).collect()
    }

    /// Delete a task
    pub fn delete(&mut self, id: &Uuid) -> Result<()> {
        let path = self.task_path(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        self.cache.remove(id);
        Ok(())
    }

    /// Load all tasks from storage
    fn load_all(&mut self) -> Result<()> {
        if !self.storage_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_task(&path) {
                    Ok(task) => {
                        self.cache.insert(task.id, task);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load task from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single task from a file
    fn load_task(&self, path: &PathBuf) -> Result<Task> {
        let content = std::fs::read_to_string(path)?;
        let task: Task = serde_json::from_str(&content)?;
        Ok(task)
    }

    /// Get the file path for a task
    fn task_path(&self, id: &Uuid) -> PathBuf {
        self.storage_dir.join(format!("{}.json", id))
    }
}

/// Task search query
#[derive(Debug, Default)]
pub struct TaskQuery {
    pub target_contains: Option<String>,
    pub task_type: Option<crate::task::TaskType>,
    pub include_completed: bool,
    pub limit: Option<usize>,
}

impl TaskQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target_contains(mut self, query: impl Into<String>) -> Self {
        self.target_contains = Some(query.into());
        self
    }

    pub fn task_type(mut self, task_type: crate::task::TaskType) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn include_completed(mut self, include: bool) -> Self {
        self.include_completed = include;
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn execute<'a>(&self, memory: &'a TaskMemory) -> Vec<&'a Task> {
        let mut results: Vec<_> = memory
            .cache
            .values()
            .filter(|task| {
                // Filter by completion status
                if !self.include_completed && task.is_complete() {
                    return false;
                }

                // Filter by target name
                if let Some(ref query) = self.target_contains {
                    if !task.target.name.to_lowercase().contains(&query.to_lowercase()) {
                        return false;
                    }
                }

                // Filter by task type
                if let Some(ref task_type) = self.task_type {
                    if &task.task_type != task_type {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by most recent
        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply limit
        if let Some(limit) = self.limit {
            results.truncate(limit);
        }

        results
    }
}
