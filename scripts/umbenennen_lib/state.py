"""Round counter of `after` (a private state file, never a symlink)."""
import os


def private_state_dir():
    """A per-user folder (mode 0700) for state files, never the shared /tmp with a predictable name."""
    base = os.environ.get("XDG_STATE_HOME") or os.path.join(os.path.expanduser("~"), ".local", "state")
    d = os.path.join(base, "freesynergy")
    os.makedirs(d, mode=0o700, exist_ok=True)
    return d


def counter(path, red):
    """Round counter in a state file; it never follows a symlink (O_NOFOLLOW), a refused file raises OSError."""
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd) as f:
            n = int(f.read().strip() or 0)
    except FileNotFoundError:
        n = 0
    except ValueError:
        n = 0
    n = n + 1 if red else 0
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as f:
        f.write(f"{n}\n")
    return n
