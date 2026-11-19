# Selenite Linux appliance

Selenite can act as a Linux appliance-style, Wayland-first application shell.
It still relies on the real Linux kernel, systemd, DBus, logind, XDG desktop
entries, the compositor, and ordinary user permissions. It never replaces init
or bypasses host authorization.

## Host window

Open **Suite > Linux Host**, or run:

```bash
selenite --host
selenite --host-status
```

The host window provides:

- Linux, kernel, uptime, memory, load, session, and battery information.
- Installed applications discovered from XDG `.desktop` files.
- User process listing and graceful `SIGTERM` termination.
- Mount and network device discovery.
- User services through `systemctl --user`.
- Notifications through the freedesktop notification stack.
- Explicit lock, logout, sleep, restart, and power actions through logind or
  systemd.
- Refresh using F5 and keyboard navigation using the number keys and arrows.

Applications can be opened by selecting one and pressing Enter. Files, folders,
and URLs use the registered Linux handler:

```bash
selenite --host-open ~/Documents
selenite --host-notify "Selenite is ready"
```

## Dedicated Wayland session

Install `cage` and Selenite, then install the display-manager session:

```bash
sudo PREFIX=/usr ./tools/install-appliance.sh
```

Select **Selenite Appliance** at the display manager. The launcher starts Selenite
through Cage as a dedicated single-application Wayland session. Inside an
existing desktop session, `selenite --host-appliance` opens the same UI maximized
without creating or replacing a compositor.

An optional systemd user service is included at
`systemd/selenite-appliance.service` for graphical sessions that intentionally
want to supervise Selenite with systemd. It is not enabled automatically.

## Security and operational policy

- No command runs through a shell.
- `.desktop` field codes are not expanded by Selenite when launching.
- Service names are validated before calling `systemctl`.
- Process termination is limited to the current user's visible processes and
  refuses to terminate the host window itself.
- Destructive power and session actions require a second Enter in the UI.
- Linux returns permission or availability errors directly; Selenite does not
  silently use `sudo` or fabricate a successful result.

Wayland is preferred by the session environment and no X11-specific window
management is required. X11 remains a compatibility fallback while an existing
display session is active.
