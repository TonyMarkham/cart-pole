# Herdr's panes on PowerShell 7.6.9

On this PC, Herdr (the terminal manager whose workspaces hold this repo's
terminals) starts every new pane in **Windows PowerShell 5.1**, the old
PowerShell built into Windows, and not in **PowerShell 7.6.9**, the one meant
to be used. This page says how that was found, the one setting that changes
it, and how to check that it took.

The two are separate programs:

| | Windows PowerShell 5.1 | PowerShell 7.6.9 |
|---|---|---|
| Program | `powershell.exe` | `pwsh.exe` |
| Where | `C:\WINDOWS\System32\WindowsPowerShell\v1.0\` (`v1.0` is the folder's name for every version up to 5.1) | Installed from the Microsoft Store (below) |
| Example difference | `a && b` is an error: "The token '&&' is not a valid statement separator in this version" | `a && b` runs `b` only if `a` worked |

`__live__/orin/rust/DEPLOY.md` is written so that its PC steps run in both.

## How it was found (2026-11-06)

Herdr reports what runs in each pane. For the three panes of the `cart-pole`
workspace:

```powershell
herdr workspace list
```

```powershell
herdr pane list --workspace <workspace id from the list>
```

```powershell
herdr pane process-info --pane <pane id from the list>
```

All three panes ran `C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe`.
That same program, run on this PC, reports its version as `5.1.26100.9711`.

PowerShell 7 on this PC, from `(Get-Command pwsh -All).Source` and
`$PSVersionTable` in a PowerShell 7 window:

- version `7.6.9`;
- launcher `C:\Users\tony\AppData\Local\Microsoft\WindowsApps\pwsh.exe`, which
  the Microsoft Store keeps pointing at the current version;
- real program
  `C:\Program Files\WindowsApps\Microsoft.PowerShell_7.6.9.0_x64__8wekyb3d8bbwe\pwsh.exe`,
  whose folder name changes with every update.

## The setting

Herdr's own default settings, printed by `herdr --default-config` (herdr-win
2026.08.31.4, Herdr 0.8.2), name the setting:

```toml
[terminal]
# Executable used for new interactive panes.
# Empty means $SHELL, then /bin/sh.
# default_shell = ""
```

On Windows, Herdr reads its settings from `%APPDATA%\herdr\config.toml`, which
on this PC is `C:\Users\tony\AppData\Roaming\herdr\config.toml`: the same folder
`herdr status` names for its socket, `herdr.sock`.

> [!WARNING]
> The default settings' own first lines say "Place this file at
> ~/.config/herdr/config.toml". That is not where Herdr on Windows reads it.
> `herdr config check` answering `config: ok` doesn't say which file it
> checked.

## Making the change

> **1. Edit `C:\Users\tony\AppData\Roaming\herdr\config.toml`.** TOML groups settings
> under headings in square brackets, and this one belongs under `[terminal]`:
>
> - If the file has a `[terminal]` line, put the `default_shell` line under it,
>   before the next `[…]` heading. If a `default_shell` line is already there,
>   replace it.
> - If the file has no `[terminal]` line, add both lines at its end.
> - If the file doesn't exist, create it with just these two lines. Herdr uses
>   its defaults for everything else.
>
> ```toml
> [terminal]
> default_shell = 'C:\Users\tony\AppData\Local\Microsoft\WindowsApps\pwsh.exe'
> ```
>
> > [!NOTE]
> > The path is the launcher, not the real program, so the setting keeps working
> > after the Store updates PowerShell. The single quotes make it a TOML literal
> > string, in which a backslash is just a backslash, so it needs no doubling.

> **2. Check the file.** Herdr reads it and prints what is wrong with it, if
> anything:
>
> ```powershell
> herdr config check
> ```

> **3. Load it into the running Herdr**, without restarting it or the panes:
>
> ```powershell
> herdr server reload-config
> ```

> **4. Open a new pane.** The setting applies only to panes opened after step 3:
> every pane already open keeps `powershell.exe`. A new tab in the `cart-pole`
> workspace brings a new pane:
>
> ```powershell
> herdr tab create --workspace <workspace id> --cwd D:\git\cart-pole --focus
> ```
>
> > [!WARNING]
> > An old pane goes away only when it is closed, and closing it ends whatever
> > runs in it. Close each one yourself, once nothing in it is needed.

## Checking it took

> **5. In the new pane, ask PowerShell its version:**
>
> ```powershell
> $PSVersionTable.PSVersion.ToString()
> ```
>
> > [!SUCCESS]
> > `7.6.9`.
>
> > [!FAILURE]
> > `5.1.26100.9711`: the pane still runs Windows PowerShell. Check that step 2
> > reported no problem, that step 3 ran, and that this pane was opened after it.

> **6. Or ask Herdr** what the new pane runs, with its id from
> `herdr pane list`:
>
> ```powershell
> herdr pane process-info --pane <pane id>
> ```
>
> > [!SUCCESS]
> > The program named is `pwsh.exe`, not `powershell.exe`.

## Restarting Herdr

Herdr is two programs. The **server** runs in the background and holds every
workspace, tab and pane, and every program running in them. The **window** is
only a view of it, attached to the server. Closing the window detaches the
view and leaves the server, and everything in its panes, running.

A changed `config.toml` doesn't need a restart: step 3's
`herdr server reload-config` loads it into the running server.

> **R1. See whether the server is running:**
>
> ```powershell
> herdr status
> ```
>
> > [!SUCCESS]
> > Under `server:`, `status: running`.

> **R2. Stop the server.**
>
> > [!CAUTION]
> > This ends every program in every pane: SSH sessions such as the one in the
> > `robo-orin` workspace, anything started in a pane, and the pane this is
> > typed in, if it is one of Herdr's. Run it from a terminal outside Herdr,
> > such as Windows Terminal, so its answer can be read.
>
> ```powershell
> herdr server stop
> ```

> **R3. Start it again and open the window.** `herdr` on its own starts the
> server when none is running, then attaches the window to it:
>
> ```powershell
> herdr
> ```

> [!NOTE]
> The workspaces, tabs and panes come back after R2 and R3, with the same ids,
> but each pane holds a new shell: whatever ran in it before is gone (seen
> 2026-11-06, every pane's shell process id changed). Their screen contents
> don't come back either: `pane_history = false` in the default settings, and
> its comment reads "Save recent pane screen history across full server
> restarts".

## Not yet verified

- **Herdr's prompt hook.** Herdr starts `powershell.exe` with a `-Command` that
  adds to the prompt, so that Herdr always knows each pane's current folder.
  Whether it does the same for `pwsh.exe` was not checked. If it doesn't, a new
  pane, tab or workspace opened from that pane may not start in the same
  folder.
- **The launcher.** That Herdr can start a pane through the Store's launcher,
  `WindowsApps\pwsh.exe`, was not tried. If step 5 fails although steps 2 and 3
  worked, the real program's path is the fallback, at the cost of editing it
  after each PowerShell update.
