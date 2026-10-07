# 0.1
- Copy global config into repository on init and use it
- Cache for version file content
  - LRU
  - cache version 20(?) steps back from the target version
  - 3 slots cache for branch tips
  - 3 slots for random checkouts
- After cache is implemented make all versions use Patch, except for root
- Tests
- Logs
- Mutex for multiple instances accessing the same repository
- Allow comparing with current file state (uncommitted)

# Backlog
- Support matching file type rules by mime type