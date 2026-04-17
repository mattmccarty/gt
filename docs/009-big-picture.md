# 009 - Big Picture

This document provides the comprehensive system overview, tying together all components into a unified architecture view.

## Table of Contents

- [System Architecture](#system-architecture)
- [Data Flow Overview](#data-flow-overview)
- [Component Interactions](#component-interactions)
- [State Management](#state-management)
- [External Integrations](#external-integrations)
- [Complete System Diagram](#complete-system-diagram)

## System Architecture

### Layered Architecture

gt follows a clean layered architecture with clear separation of concerns:

```mermaid
graph TB
    subgraph "Presentation Layer"
        CLI[CLI Parser<br/>clap]
        OUTPUT[Output Formatter<br/>terminal/JSON/CSV]
        INTERACT[Interactive UI<br/>dialoguer]
    end

    subgraph "Application Layer"
        CMD_INIT[Init Command]
        CMD_SCAN[Scan Command]
        CMD_ADD[Add Command]
        CMD_LIST[List Command]
        CMD_SWITCH[Switch Command]
        CMD_CLONE[Clone Command]
        CMD_CONFIG[Config Command]
        CMD_MIGRATE[Migrate Command]
        CMD_FIX[Fix Command]
        CMD_KEY[Key Command]
        CMD_STATUS[Status Command]
    end

    subgraph "Domain Layer"
        IDENTITY[Identity Model]
        REPO[Repository Model]
        PROVIDER[Provider Model]
        URL[URL Parser]
    end

    subgraph "Strategy Layer"
        STRAT_TRAIT[Strategy Trait]
        SSH_ALIAS[SSH Alias]
        CONDITIONAL[Conditional]
        URL_REWRITE[URL Rewrite]
    end

    subgraph "Infrastructure Layer"
        SSH_CFG[SSH Config I/O]
        GIT_CFG[Git Config I/O]
        SSH_KEY[SSH Key Manager]
        BACKUP[Backup Manager]
        CONFIG[Config Manager]
    end

    subgraph "External Systems"
        FS[(File System)]
        SSH_AGENT[SSH Agent]
        GIT[Git CLI]
        PROVIDERS[Git Providers]
    end

    CLI --> CMD_INIT
    CLI --> CMD_SCAN
    CLI --> CMD_ADD

    CMD_INIT --> IDENTITY
    CMD_SCAN --> REPO
    CMD_ADD --> IDENTITY

    IDENTITY --> STRAT_TRAIT
    STRAT_TRAIT --> SSH_ALIAS
    STRAT_TRAIT --> CONDITIONAL
    STRAT_TRAIT --> URL_REWRITE

    SSH_ALIAS --> SSH_CFG
    CONDITIONAL --> GIT_CFG
    URL_REWRITE --> GIT_CFG

    SSH_CFG --> FS
    GIT_CFG --> FS
    SSH_KEY --> FS
    SSH_KEY --> SSH_AGENT

    OUTPUT --> CLI
    INTERACT --> CLI
```

### Module Dependencies

```mermaid
graph LR
    subgraph "Entry Point"
        MAIN[main.rs]
    end

    subgraph "CLI"
        ARGS[cli::args]
        OUT[cli::output]
        INT[cli::interactive]
    end

    subgraph "Commands"
        CMDS[cmd::*]
    end

    subgraph "Core"
        CORE_ID[core::identity]
        CORE_REPO[core::repo]
        CORE_URL[core::url]
        CORE_PATH[core::path]
        CORE_PROV[core::provider]
    end

    subgraph "Strategy"
        STRAT[strategy::*]
    end

    subgraph "I/O"
        IO_SSH[io::ssh_config]
        IO_GIT[io::git_config]
        IO_KEY[io::ssh_key]
        IO_BAK[io::backup]
        IO_TOML[io::toml_config]
    end

    subgraph "Scanning"
        SCAN[scan::*]
    end

    subgraph "Shared"
        ERR[error]
        UTIL[util]
    end

    MAIN --> ARGS
    MAIN --> OUT
    ARGS --> CMDS

    CMDS --> CORE_ID
    CMDS --> CORE_REPO
    CMDS --> STRAT
    CMDS --> SCAN

    STRAT --> IO_SSH
    STRAT --> IO_GIT
    STRAT --> CORE_URL

    SCAN --> IO_SSH
    SCAN --> IO_GIT

    IO_SSH --> IO_BAK
    IO_GIT --> IO_BAK
    IO_KEY --> CORE_PATH

    CORE_ID --> ERR
    CORE_REPO --> ERR
    IO_SSH --> ERR
```

## Data Flow Overview

### Identity Creation Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant AddCmd
    participant Identity
    participant Strategy
    participant KeyMgr as SSH Key Manager
    participant SshCfg as SSH Config
    participant GitCfg as Git Config
    participant Backup

    User->>CLI: gt config id add work --email work@co.com
    CLI->>AddCmd: Execute with options

    AddCmd->>Identity: Create identity model
    Identity-->>AddCmd: Identity validated

    AddCmd->>Strategy: Get configured strategy
    Strategy-->>AddCmd: SSH Alias Strategy

    AddCmd->>KeyMgr: Generate SSH key
    KeyMgr->>KeyMgr: ssh-keygen -t ed25519
    KeyMgr->>KeyMgr: Set permissions 0600
    KeyMgr-->>AddCmd: Key path

    AddCmd->>Backup: Backup SSH config
    Backup->>Backup: Create timestamped backup
    Backup-->>AddCmd: Backup created

    AddCmd->>SshCfg: Add host entry
    SshCfg->>SshCfg: Parse existing config
    SshCfg->>SshCfg: Add new Host block
    SshCfg-->>AddCmd: Config updated

    AddCmd->>GitCfg: Save to gt id config
    GitCfg-->>AddCmd: Config saved

    AddCmd-->>CLI: Success
    CLI-->>User: Identity 'work' created
```

### Repository Switch Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SwitchCmd
    participant Repo
    participant Config
    participant Strategy
    participant URL
    participant Git

    User->>CLI: gt id switch work
    CLI->>SwitchCmd: Execute

    SwitchCmd->>Repo: Detect current repository
    Repo->>Git: git rev-parse --show-toplevel
    Git-->>Repo: /path/to/repo
    Repo->>Git: git remote get-url origin
    Git-->>Repo: git@github.com:org/repo.git
    Repo-->>SwitchCmd: Repository info

    SwitchCmd->>Config: Load identity 'work'
    Config-->>SwitchCmd: Identity loaded

    SwitchCmd->>Strategy: Get strategy for identity
    Strategy-->>SwitchCmd: SSH Alias Strategy

    SwitchCmd->>URL: Transform URL
    URL->>URL: Parse original URL
    URL->>URL: Apply identity prefix
    URL-->>SwitchCmd: git@gt-work.github.com:org/repo.git

    SwitchCmd->>Git: git remote set-url origin <new>
    Git-->>SwitchCmd: URL updated

    SwitchCmd-->>CLI: Success
    CLI-->>User: Switched to 'work'
```

### Scan and Detection Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant ScanCmd
    participant SshScanner
    participant GitScanner
    participant Detector
    participant Report

    User->>CLI: gt id scan
    CLI->>ScanCmd: Execute

    par SSH Scanning
        ScanCmd->>SshScanner: Scan SSH config
        SshScanner->>SshScanner: Parse ~/.ssh/config
        SshScanner->>SshScanner: Find Host blocks
        SshScanner->>SshScanner: Detect gt patterns
        SshScanner-->>ScanCmd: SSH entries
    and Git Scanning
        ScanCmd->>GitScanner: Scan Git config
        GitScanner->>GitScanner: Parse ~/.gitconfig
        GitScanner->>GitScanner: Find includeIf entries
        GitScanner->>GitScanner: Find url.*.insteadOf
        GitScanner-->>ScanCmd: Git entries
    end

    ScanCmd->>Detector: Analyze entries
    Detector->>Detector: Classify by strategy
    Detector->>Detector: Match identities
    Detector->>Detector: Find orphans
    Detector-->>ScanCmd: Detection results

    ScanCmd->>Report: Generate report
    Report->>Report: Format for output
    Report-->>ScanCmd: Formatted report

    ScanCmd-->>CLI: Report
    CLI-->>User: Display results
```

## Component Interactions

### Strategy Selection

```mermaid
flowchart TD
    START[Need Strategy] --> CHECK{Identity has<br/>strategy override?}

    CHECK -->|Yes| OVERRIDE[Use identity strategy]
    CHECK -->|No| DEFAULT[Use default strategy]

    OVERRIDE --> VALIDATE{Strategy valid<br/>for provider?}
    DEFAULT --> VALIDATE

    VALIDATE -->|Yes| CREATE[Create strategy instance]
    VALIDATE -->|No| FALLBACK[Fall back to SSH alias]

    FALLBACK --> CREATE

    CREATE --> DONE[Return strategy]
```

### Configuration Resolution

```mermaid
flowchart TD
    START[Load Configuration] --> CLI{CLI args?}

    CLI -->|Yes| CLI_VAL[Use CLI value]
    CLI -->|No| ENV{Env var?}

    ENV -->|Yes| ENV_VAL[Use env value]
    ENV -->|No| PROJECT{Project config?}

    PROJECT -->|Yes| PROJ_VAL[Use project value]
    PROJECT -->|No| USER{User config?}

    USER -->|Yes| USER_VAL[Use user value]
    USER -->|No| DEF_VAL[Use default]

    CLI_VAL --> MERGE[Merge into final config]
    ENV_VAL --> MERGE
    PROJ_VAL --> MERGE
    USER_VAL --> MERGE
    DEF_VAL --> MERGE

    MERGE --> VALIDATE[Validate config]
    VALIDATE --> DONE[Return config]
```

### Backup and Recovery

```mermaid
flowchart TD
    START[Modify File] --> BACKUP{Backup enabled?}

    BACKUP -->|No| MODIFY[Modify file directly]
    BACKUP -->|Yes| COUNT[Count existing backups]

    COUNT --> MAX{At max backups?}

    MAX -->|Yes| ROTATE[Delete oldest backup]
    MAX -->|No| CREATE[Create new backup]

    ROTATE --> CREATE
    CREATE --> PERMS[Set secure permissions]
    PERMS --> MODIFY

    MODIFY --> VERIFY{Modification success?}

    VERIFY -->|Yes| DONE[Complete]
    VERIFY -->|No| RESTORE[Restore from backup]

    RESTORE --> ERROR[Report error]
```

## State Management

### Identity State Machine

```mermaid
stateDiagram-v2
    [*] --> NotConfigured: User starts

    NotConfigured --> Partial: gt config id add (no key)
    NotConfigured --> Ready: gt config id add (with key gen)

    Partial --> Ready: gt config id key generate
    Partial --> NotConfigured: gt remove

    Ready --> Active: gt id switch
    Ready --> Ready: gt config id key test (pass)
    Ready --> Error: gt config id key test (fail)

    Active --> Ready: gt id switch (different)
    Active --> Active: git operations

    Error --> Ready: Fix SSH key
    Error --> Partial: Remove key

    Ready --> NotConfigured: gt remove
    Active --> NotConfigured: gt remove
```

### Repository State

```mermaid
stateDiagram-v2
    [*] --> Unknown: Detect repository

    Unknown --> Standard: Original URL format
    Unknown --> Modified: gt URL format
    Unknown --> NotRepo: Not a git repo

    Standard --> Modified: gt id switch
    Modified --> Standard: gt id fix --restore
    Modified --> Modified: gt id switch (different identity)

    Standard --> [*]: Exit
    Modified --> [*]: Exit
    NotRepo --> [*]: Error
```

## External Integrations

### Git Provider Communication

```mermaid
flowchart LR
    subgraph "gt"
        KEY[SSH Key]
        SSH[SSH Config]
    end

    subgraph "SSH Layer"
        AGENT[SSH Agent]
        CLIENT[SSH Client]
    end

    subgraph "Providers"
        GH[GitHub]
        GL[GitLab]
        BB[Bitbucket]
        AZ[Azure DevOps]
        CC[CodeCommit]
        CUSTOM[Custom]
    end

    KEY --> AGENT
    SSH --> CLIENT
    AGENT --> CLIENT

    CLIENT --> GH
    CLIENT --> GL
    CLIENT --> BB
    CLIENT --> AZ
    CLIENT --> CC
    CLIENT --> CUSTOM
```

### File System Layout

```mermaid
flowchart TD
    subgraph "Home Directory"
        HOME[~]

        subgraph ".ssh"
            SSH_CFG[config]
            SSH_KEYS[id_gt_*]
            KNOWN[known_hosts]
        end

        subgraph ".config/gt"
            GITID_CFG[config.toml]
            GITID_BAK[*.bak]
        end

        subgraph ".gitconfig*"
            GIT_CFG[.gitconfig]
            GIT_INCLUDES[.gitconfig.d/*]
        end

        HOME --> .ssh
        HOME --> .config/gt
        HOME --> .gitconfig*
    end

    subgraph "Repository"
        REPO_ROOT[repo/]
        DOT_GIT[.git/]
        DOT_GITID[..gt.toml]
        REPO_CFG[.git/config]

        REPO_ROOT --> DOT_GIT
        REPO_ROOT --> DOT_GITID
        DOT_GIT --> REPO_CFG
    end
```

## Complete System Diagram

This diagram shows the entire gt system with all components, data flows, and external integrations:

```mermaid
flowchart TB
    subgraph USER["User Interface"]
        direction LR
        TERM[Terminal]
        SCRIPT[Scripts/CI]
    end

    subgraph CLI_LAYER["CLI Layer"]
        direction TB
        CLAP[Argument Parser<br/>clap]
        DIALOG[Interactive Prompts<br/>dialoguer]
        FORMAT[Output Formatter<br/>terminal/JSON/CSV]
    end

    subgraph CMD_LAYER["Command Layer"]
        direction TB
        subgraph SETUP["Setup Commands"]
            INIT[init]
            SCAN[scan]
            ADD[add]
        end
        subgraph DAILY["Daily Commands"]
            LIST[list]
            SWITCH[switch]
            CLONE[clone]
            STATUS[status]
        end
        subgraph MAINT["Maintenance"]
            CONFIG[config]
            MIGRATE[migrate]
            FIX[fix]
            KEY[key]
        end
    end

    subgraph CORE_LAYER["Core Domain"]
        direction TB
        ID_MODEL[Identity Model]
        REPO_MODEL[Repository Model]
        PROV_MODEL[Provider Model]
        URL_PARSE[URL Parser]
        PATH_UTIL[Path Utilities]
    end

    subgraph STRAT_LAYER["Strategy Layer"]
        direction TB
        STRAT_TRAIT[Strategy Trait]
        SSH_STRAT[SSH Alias<br/>Strategy]
        COND_STRAT[Conditional<br/>Strategy]
        URL_STRAT[URL Rewrite<br/>Strategy]
    end

    subgraph IO_LAYER["I/O Layer"]
        direction TB
        SSH_PARSE[SSH Config<br/>Parser/Writer]
        GIT_PARSE[Git Config<br/>Parser/Writer]
        KEY_MGR[SSH Key<br/>Manager]
        TOML_MGR[TOML Config<br/>Manager]
        BACKUP_MGR[Backup<br/>Manager]
    end

    subgraph SCAN_LAYER["Detection Layer"]
        direction TB
        DETECTOR[Strategy Detector]
        SSH_SCAN[SSH Scanner]
        GIT_SCAN[Git Scanner]
        REPORT[Report Generator]
    end

    subgraph EXTERN["External Systems"]
        direction TB
        subgraph FILES["File System"]
            SSH_CFG[(~/.ssh/config)]
            SSH_KEYS[(SSH Keys)]
            GIT_GLOBAL[(~/.gitconfig)]
            GITID_CFG[(config.toml)]
            REPO_CFG[(.git/config)]
        end
        subgraph TOOLS["System Tools"]
            SSH_KEYGEN[ssh-keygen]
            SSH_ADD[ssh-add]
            SSH_AGENT[ssh-agent]
            GIT_CLI[git]
        end
        subgraph PROVIDERS["Git Providers"]
            GITHUB[GitHub]
            GITLAB[GitLab]
            BITBUCKET[Bitbucket]
            AZURE[Azure DevOps]
            CODECOMMIT[CodeCommit]
            CUSTOM_PROV[Custom]
        end
    end

    %% User to CLI
    TERM --> CLAP
    SCRIPT --> CLAP
    DIALOG --> TERM
    FORMAT --> TERM

    %% CLI to Commands
    CLAP --> INIT
    CLAP --> SCAN
    CLAP --> ADD
    CLAP --> LIST
    CLAP --> SWITCH
    CLAP --> CLONE
    CLAP --> STATUS
    CLAP --> CONFIG
    CLAP --> MIGRATE
    CLAP --> FIX
    CLAP --> KEY

    %% Commands to Core
    INIT --> ID_MODEL
    ADD --> ID_MODEL
    SWITCH --> ID_MODEL
    SWITCH --> REPO_MODEL
    CLONE --> URL_PARSE
    FIX --> URL_PARSE
    SCAN --> DETECTOR

    %% Core to Strategy
    ID_MODEL --> STRAT_TRAIT
    STRAT_TRAIT --> SSH_STRAT
    STRAT_TRAIT --> COND_STRAT
    STRAT_TRAIT --> URL_STRAT

    %% Strategy to I/O
    SSH_STRAT --> SSH_PARSE
    SSH_STRAT --> KEY_MGR
    COND_STRAT --> GIT_PARSE
    URL_STRAT --> GIT_PARSE

    %% I/O to Files
    SSH_PARSE --> SSH_CFG
    SSH_PARSE --> BACKUP_MGR
    GIT_PARSE --> GIT_GLOBAL
    GIT_PARSE --> REPO_CFG
    GIT_PARSE --> BACKUP_MGR
    KEY_MGR --> SSH_KEYS
    KEY_MGR --> SSH_KEYGEN
    KEY_MGR --> SSH_ADD
    TOML_MGR --> GITID_CFG
    BACKUP_MGR --> SSH_CFG
    BACKUP_MGR --> GIT_GLOBAL

    %% Scanning
    DETECTOR --> SSH_SCAN
    DETECTOR --> GIT_SCAN
    SSH_SCAN --> SSH_CFG
    GIT_SCAN --> GIT_GLOBAL
    DETECTOR --> REPORT
    REPORT --> FORMAT

    %% External connections
    SSH_AGENT --> GITHUB
    SSH_AGENT --> GITLAB
    SSH_AGENT --> BITBUCKET
    SSH_AGENT --> AZURE
    SSH_AGENT --> CODECOMMIT
    SSH_AGENT --> CUSTOM_PROV

    %% Styling
    style USER fill:#e1f5fe
    style CLI_LAYER fill:#f3e5f5
    style CMD_LAYER fill:#fff3e0
    style CORE_LAYER fill:#e8f5e9
    style STRAT_LAYER fill:#fce4ec
    style IO_LAYER fill:#fff8e1
    style SCAN_LAYER fill:#e0f7fa
    style EXTERN fill:#f5f5f5
```

## Summary

gt is designed as a modular, extensible CLI application with:

1. **Clean separation of concerns** across layers
2. **Strategy pattern** for flexible identity management
3. **Cross-platform support** through abstracted I/O
4. **Safety-first approach** with backups and dry-run modes
5. **Extensible architecture** for new providers and strategies

The system handles the complete lifecycle of Git identity management:
- Detection of existing configurations
- Creation and management of identities
- Switching between identities per-repository
- Migration between strategies
- Repair and maintenance of configurations

All components are designed to work together seamlessly while remaining independently testable and maintainable.
