# BDGM Player arch

This document describes the design of bdgm-play.

bdgm-play has:

- main.rs - calls app::run
- lib.rs - refs every module in the app.

definitions:

- image: a .bin, .udf or other binary image of a BDGM disc (UDF 2.50/2.01)
- mounted disc: a disc already read by the system and mounted to a directory (or an unbuilt source dir)
- raw disc: a disc not read by the system, referenced by its path (Windows: '\\.\X:') that should be extracted by the player.
- atomically write: Write into file/directory named "\*.part" and rename after writing.
- AppDirs: platform_dirs::AppDirs

**Only items defined in this document may be exported outside of their module. Helper functions added to supplement these should be declared in the same module, and may not be pub.**  
In the resulting Rust code, pub and async should be used as needed.

## app

- run() Parses args (using clap) and calls run_gui or run_cli accordingly. If LOCATION argument is not specified, and --raw-disc or --image flags are not enabled, use gui.  
   If LOCATION is specified, use cli.  
   If LOCATION is not specified and either --raw-disc or --image are enabled, display an error stating that no location was specified and halt.  
  If runtime is specified, normalize it such that relative paths become absolute but PATH-resolved paths stay PATH-resolved.

## args

Contains the Args struct (parseable via clap):

- location: Option<PathBuf> - determines the location of the source. Option because GUI doesn't read this.
- image: bool - flag - whether there is an image at the location.
- raw-disc: bool - flag - ONLY ON WINDOWS - whether there is a raw disc at the location.
- runtime: Option<PathBuf> - argument - specifies the location of the runtime to use to run the app, instead of the default one. CLI and GUI read this.

Args should not go any further than to run_gui or run_cli.

## cli

run_cli(args) - Parses args into a LaunchRequest and launches it via launch_request.

CliStateHandler (implements StateHandler)

- change_state - println!s State
- revert_state - no-op (CLI has no transient state to revert).

## gui

Displays a GUI with two buttons: Open Disc, Open Image

- Open Disc: opens either:
  - a mounted disc on non-Windows platforms
  - a raw disc on Windows platforms
- Open Image: opens an image.

Opening a mounted disc or image shows a system file/directory picker.

Opening a raw disc queries all attached drives and lists all disc drives (if any; if none, list all drives).  
The drives will be listed in a modal replacing the main view until a selection is made (see below)

### app

run_gui(args) Hides console if bdgm-play wasn't run by a shell, launches iced window and sets AppState.runtime from args. If the app errors out, eprintln!s the error and, if the shell was hidden, creates an error dialog.  
 When closing the window, check if the current AppState is Playing. if Playing(HtmlServer), kill the server. If Playing(GameProcess), let the game run after closing.

hide_console_if_no_shell() -> bool - On Windows, checks if the number of processes in the current shell is exactly 1 (no pwsh, only bdgm-play), and, if so (not 0 or >= 2), hides the shell. Returns whether the shell was hidden. On non-Windows, does nothing and returns false.  
Hides console instead of destroying it, because spawned games inherit stdout/stderr; starting would fail if it is destroyed.

### state

Contains the application's current state in multiple structs.

enum Status:

- Idle(Option<(ValidatedGame, Option<i32>)>) - contains the last exited game and its exit code.
- Starting(Arc<State>)
- Playing(ValidatedGame, LaunchHandle)
- Error(Arc<AppError>)

struct AppState:

- runtime: Option<PathBuf> - from args, replaces default runtime.
- status: Status - the current status of core.
- disc_picker_discs: Option<Vec<char>> - if None, raw disc picker is not open.

### message

enum Message:

- OpenDiscClicked
- OpenImageClicked
- DiscPickerCancel
- RunRawDisc(char)
- RunMounted(PathBuf)
- RunImage(PathBuf)
- StateChanged(Arc<State>)
- GameStarted(ValidatedGame, LaunchHandle)
- StopGameClicked
- OpenHTMLGameClicked
- GameExited(ValidatedGame, Result<Option<i32>>)
- GameError(AppError)
- None - used for canceled file pickers.

### view

Contains the app's views.

- view(&AppState) -> iced::Element - displays either:
  - disc_picker if disc_picker_discs is Some
  - main_view otherwise if status is Idle or Error
    - error_view under the above if Status is Error
  - starting_view if status is Starting
  - playing_view if status is Playing

- main_view(&AppState) -> iced::Element - displays, side by side, "Open Disc" and "Open Image" buttons, and if Some, shows the last game's name and exit code (if exit code is Some; do not display exit code if exit code is 0): "Example Game exited"; "Example Game exited with code 1"

- disc_picker(&AppState) -> iced::Element - lists buttons for each entry in disc_picker_discs and a cancel button.  
  Clicking a disc button sends RunRawDisc(char of the disc)  
  On non-Windows, displays "Raw disc is only supported on Windows" and the cancel button.

- starting_view(&State) -> iced::Element - displays State.

- playing_view(&ValidatedGame, &LaunchHandle) -> iced::Element - displays the currently playing game information,  
  and, in a row, the Stop button, and (if HTML game), the Open button (which opens the game in browser).

- error_view(&AppError) -> iced::Element - displays AppError in red text.

#### raw_disc (#[cfg(windows)])

Contains logic for getting disc drive letters.

- get_disc_drives() -> Vec<char> - Gets all CDROM disc drives. If there are none, gets all attached drives.

### update

Most branches will return a Task::none, unless they send another Message or launch a background service (file picker, game launch).

update(&mut AppState, Message) -> Task<Message> - calls other update functions based on Message. On RunMounted/Image/RawDisc, creates a LaunchRequest, calls pre_launch and launch. Returns Task::none on None.

#### picker

Opens a file, directory or disc picker.

on_open_disc_clicked(&mut AppState) -> Task<Message>  
 if Windows (use if(cfg!(windows)) and only use #[cfg(windows)] on statements that need it), gets disc drive letters and moves them to AppState.  
 if not windows, creates and displays a directory picker in a new task and fires RunMounted afterwards if selected. Fires Task::none instead if picker is cancelled.

on_disc_picker_cancel(&mut AppState) -> Task<Message> - sets disc_picker_discs to None.

on_open_image_clicked(&mut AppState) -> Task<Message> - creates and displays a file picker for BDGM image files (.bin, .udf, .iso), fires RunImage if selected. Fires Task::none instead if picker is cancelled.

#### launch

create_image_launch_request(&AppState, PathBuf) -> LaunchRequest

create_mounted_launch_request(&AppState, PathBuf) -> LaunchRequest

create_raw_disc_launch_request(&AppState, char) -> LaunchRequest - converts char to path ("\\.\X:")

launch(LaunchRequest) -> Task<Message>
creates a new Task::stream(stream::channel) that: wraps iced::Sender in a tokio UnboundedSender + UnboundedReceiver, forwarding from the UnboundedReceiver to the iced::Sender in a tokio::spawn, creates new GuiStateHandler using that UnboundedSender,  
launches LaunchRequest in a tokio::spawn_blocking, fires GameStarted. If an AppError arises, fires GameError and stops.  
 Then, if GameProcess, waits for game exit, converts Result<Option<ExitStatus>> to Result<Option<i32>>, and fires GameExited.

pre_launch(&mut AppState) - Sets disc_picker_discs to None, sets status to Starting(Arc::new(StartingExtraction))

#### state

Updates status based on State from a launching game.

struct GuiStateHandler (implements StateHandler)

- current_state: Option<Arc<State>>
- last_state: Option<Arc<State>>
- sender: (tokio::sync::mpsc::UnboundedSender<Message>)
- change_state - Moves current_state to last_state, saves new current_state and sends StateChanged.
- revert_state - If last_state is Some, swaps current_state and last_state and sends StateChanged.
- new(tokio::sync::mpsc::UnboundedSender<Message>) -> Self

on_state_changed(&mut AppState, Arc<State>) -> Task<Message> - If Status is Starting, changes status to Error or Starting(State), based on whether State is Error.

#### running

on_game_started(&mut AppState, ValidatedGame, LaunchHandle) -> Task<Message> - Sets status to Playing.

on_open_html_game_clicked(&AppState) -> Task<Message> - if current status is Playing(HtmlServer), opens the game in a browser.

on_stop_game_clicked(&mut AppState) -> Task<Message> - if current status is Playing, kills the currently playing game. Then, if GameProcess, GameExited will be called by the wait in launch; if HTML, sets status to Idle(Some((ValidatedGame, None))).

on_game_exited(&mut AppState, ValidatedGame, exit_code: Result<Option<i32>>) -> Task<Message> - if exit_code is Err, calls on_error. If Ok, sets status to Idle(Some((ValidatedGame, exit_code))).

on_error(&mut AppState, AppError) -> Task<Message> - sets state to Error.

## core

This module includes all "business logic" of bdgm-play, as described below.  
The goal of core is to provide APIs for launching a launch request.

pub use everything marked with pub in this document. Pub in this document does not literally mean that the thing should be marked as pub in Rust, just that it will be re-exported in core/mod.rs.

### request

This module contains LaunchRequest and related structs.

pub struct LaunchRequest:

- source: Source
- runtime: Option<PathBuf>
- pub new(Source, Option<PathBuf>) -> Self

pub struct Source:

- path: PathBuf
- kind: SourceKind
- pub new(PathBuf, SourceKind) -> Self

pub enum SourceKind:

- MountedDisc
- RawDisc (do not cfg-gate; impl for Unix can be added in the future and for now, the gui or cli should not construct this outside Windows; handlers calling into cfg-gated code return UnsupportedOnThisPlatform if not on Windows)
- Image

enum ResolvedSource

- Extracted(ExtractedSource) - an extracted directory from a source, to be installed.
- Installed(InstalledGame) - a Source that has already been installed and does not need to be extracted.

struct ExtractedSource - must be dropped after installation.

- path: PathBuf - the location of the extracted dir's root.
- \_tempdir: Option<tempfile::TempDir> - holds a temp dir until installed, to delete when dropped.
- validate(&self, &mut dyn StateHandler) -> Result<ValidatedSource> - Calls functions from validate and creates a ValidatedSource from the returned bdgm::ValidatedGame. Emits Validating to StateHandler.

struct ValidatedSource

- game: bdgm::ValidatedGame
- path: PathBuf
- \_tempdir: Option<tempfile::TempDir>
- install(Self, &AppDirs, &mut dyn StateHandler) -> Result<InstalledGame> - calls functions from install and creates an InstalledGame from the returned GameDirs. Emits Installing to StateHandler.

struct InstalledGame

- game: bdgm::ValidatedGame
- game_dirs: GameDirs - the game's installation and data dir

### resolve

This module resolves a Source. If the game's version's install dir already exists and executable exists inside it, skip extraction and return InstalledGame.  
resolve_image/raw_disc/mounted parse BDGM/DISC.BDGM into bdgm::Game and validate to bdgm::ValidatedGame (using get_udf/mounted_manifest from the related module); if Ok and version is already installed, create and return Installed, otherwise extract.  
If manifest could not be read or parsed, extract regardless of whether it is already installed. If manifest could not be validated return a ValidationError.

resolve(Source, &AppDirs, &mut dyn StateHandler) -> Result<ResolvedSource> - uses functions below to extract a Source and returns a ResolvedSource. Source is consumed. Emits Extracting to StateHandler.

resolve_image(&Path, &AppDirs, &mut dyn StateHandler) -> Result<ResolvedSource> - extracts an Image.

resolve_raw_disc(&Path, &AppDirs, &mut dyn StateHandler) -> Result<ResolvedSource> - Returns UnsupportedOnThisPlatform outside Windows.

resolve_mounted(&Path, &AppDirs, &mut dyn StateHandler) -> Result<ResolvedSource> - Points ExtractedSource at itself (since the disc is already mounted on the filesystem, there's no other fs to extract from)

#### raw_disc (cfg(windows) when linking mod; callers should check with if(cfg!(windows)) and return UnsupportedOnThisPlatform instead of compile-time #[cfg(windows)])

Contains a RawDiscReader that wraps File and reads it in fixed sectors, and logic to check and extract the disc.

SECTOR_SIZE = 2048

RawDiscReader (implements Seek, Read):

- file: File
- pos: u64,
- len: u64,
- cache_sector: u64,
- cache: [u8; SECTOR_SIZE],
- cache_valid: bool,
- impl TryFrom<File> - creates a new reader from a file. Uses disc_size. Error is std::io::Error
- open(&Path) -> Result<Self> - creates a new reader from a path to the disc. Returns an error if the file could not be opened or if the path is not a disc.

disc_size(&File) -> io::Result<u64> - Uses unsafe DeviceIoControl with IOCTL_STORAGE_READ_CAPACITY to get the written size of the disc.

#### udf

open_udf_volume<T: Read + Seek>(T) -> Result<UdfVolume<T>>

get_udf_manifest(&UdfVolume<impl Read + Seek>) -> Result<Option<bdgm::Game>> (not ValidatedGame)

extract(&UdfVolume<impl Read + Seek>, &mut dyn StateHandler) -> Result<ExtractedSource> - uses extract_dir.

extract_dir(&UdfVolume<impl Read + Seek>, &UdfDir, &Path, current_size: &mut u64, total_size: &u64, &mut dyn StateHandler) -> Result<()> - recursive  
Skip all elements escaping the UDF volume - reject all UdfDirs with paths that are not single-component, not absolute, and not current dir or parent dir.

#### mounted

get_mounted_manifest(&Path) -> Result<Option<bdgm::Game>>

### validate

Validates an ExtractedSource.

validate(source: ExtractedSource) -> Result<bdgm::ValidatedGame> - opens and validates the manifest from the extracted source and checks if executable exists.

executable_exists(&ValidatedGame, app_path : &PathBuf) -> bool - Checks if the executable exists within the specified BDGM/APP dir

### install

Installs a validated extracted source.

install(source: ValidatedSource, &AppDirs, &mut dyn StateHandler) -> Result<GameDirs> - Creates game dirs, then installs the game into the install dir specified in the tree under dirs and returns the game's dirs.  
Wait for lock (install_dir.lock), then check: if install dir and executable exists, skip installation. If install dir exists but executable doesn't, then remove and overwrite install dir.  
Should atomically write the installation directory.  
If a stale install_dir.part exists after locking before installation, that means an installation was interrupted; remove existing part and proceed as if it didn't exist.

check_installed(id: &str, version: &str, &AppDirs, &mut dyn StateHandler) -> Result<bool> - Checks whether the installation directory of the specified version of the ID exists.  
Should check if parent exists, then lock install_dir.lock when checking (wait for lock).

### dirs

Constructs directory paths.

bdgm-play follows this tree:

- AppDirs::data_dir/
  - ports.json
  - games/
    - game_id/ - GameDirs root
      - app/
        - version/ - copied APP/ from disc (the install dir)
      - data/ - BDGM_DATA envvar
- AppDirs::cache_dir/
  - games/
    - game_id/ - BDGM_CACHE envvar

struct GameDirs:

- app_dirs: Arc<AppDirs>
- id: String
- version: String
- executable: PathBuf - relative to install dir
- from(&bdgm::ValidatedGame, Arc<AppDirs>) - gets id, version and executable path from ValidatedGame
- root(&self) -> PathBuf
- install_dir(&self) -> PathBuf
- cache(&self) -> PathBuf
- data(&self) -> PathBuf
- executable(&self) -> PathBuf - gets absolute path to executable (joining install_dir and executable)
- id(&self) -> &str

get_install_dir_path(id: &str, version: &str, &AppDirs) -> PathBuf - creates the install dir path for GameDirs::install_dir and install::check_installed

get_disc_manifest_path(&Path) -> PathBuf - gets path to BDGM/DISC.BDGM with a disc root path

get_disc_app_dir_path(&Path) -> PathBuf - gets path to BDGM/APP/ with a disc root path

get_app_dirs() -> Result<AppDirs> - gets app dir paths (maps None to NoAppDirs error)

create_game_dirs(&GameDirs) -> io::Result<()> - creates game data, cache and parent of install_dir specified in the tree under dirs if they don't exist.

### fs

Helpers for manipulating files and directories.

acquire_lock(&Path, blocking: bool, Option<&mut dyn StateHandler>) -> std::io::Result<File> - opens (path).lock and locks it; if blocking = true, tries lock and if would block, sets state to WaitingForLock(lock file path) (if StateHandler is Some) and then resets back.

open_file(&Path) -> io::Result<File> - simple wrapper around a file options builder for writable file open

open_part_file(&Path) -> io::Result<File> - Opens (path).part

truncate(&mut File) -> io::Result<()> - clears file

### launch

This module launches a LaunchRequest.

pub launch_request(LaunchRequest, on_state: &mut dyn StateHandler) -> Result<(ValidatedGame, LaunchHandle)> - resolves, validates, installs (if not already installed) and runs a launch request. This is called by the client.

run_game(&InstalledGame, runtime: Option<&Path>, &mut dyn StateHandler) -> Result<LaunchHandle> - either runs HTML server or creates game data and cache dirs and builds and runs command.

#### handle

pub enum LaunchHandle (implements Clone)

- GameProcess(GameProcess)
- HtmlServer(Arc<HtmlServer>)
- kill(&self) - kills LaunchHandle.
- wait will only be called by matching and calling wait on GameProcess (HtmlServer would hang indefinitely)

#### server

pub struct HtmlServer - contains server information:

- port: u16
- entry_point: String - the URI to the entry point of the server
- abort: tokio::task::AbortHandle
- start(dir: &Path, port: u16) -> Result<Self> - starts a new server at 127.0.0.1:port. If the port is already taken, error with PortTaken.
- get_url(&self) -> &str
- open(&self) - opens the entry point in a browser
- kill(&self) - aborts the AbortHandle

unique_port(id: &str, &mut dyn StateHandler) -> Result<u16> - gets persisted unique port for this game ID (stored in data_dir/ports.json), or creates one (no two IDs may share a port)  
ports.json must be written atomically and wait for lock.  
Creates ports.json parent dir if it doesn't exist.

start_server(&GameDirs, entry_point: String, &mut dyn StateHandler) -> Result<HtmlServer> - gets paths and ID from GameDirs, starts a server for install_dir and opens in browser, creating a unique port in the process  
The executable path is the path to the index endpoint from the root of APP (installed into install_dir)

#### process

pub struct GameProcess(Arc<tokio::Mutex<tokio::process::Child>>) (implements Clone)

- kill(&self) - starts killing process (doesn't wait)
- wait(self) -> Result<Option<ExitStatus>> - waits for process to end and returns its exit status. Instead of awaiting it periodically acquires lock and checks if process exited. Maps io::Error from the child process to RunningError.

get_envvars(&InstalledGame) -> HashMap<&'static str, String> - gets environment vars as specified in the spec. Use InstalledGame.game.bdgm_version for BDGM_VERSION. See the tree under dirs.

build_command(&InstalledGame, runtime: Option<&Path>) -> Result<tokio::process::Command> - calls get_envvars; matches the game's runtime field, uses build_bin_command (calls function executable from GameDirs); the HTML branch should be unreachable as that is checked in run_game

build_bin_command(default_runtime: &Path, executable: &Path, install_dir: &Path, runtime: Option<&Path>, runtime_args: &[&str], args: &[&str], envvars: &HashMap<&'static str, String>) -> Result<tokio::process::Command> - generates a command.
Sets working directory to executable.parent

run_command(tokio::process::Command, &bdgm::Runtime) -> Result<LaunchHandle> - Maps io::NotFound to RuntimeNotFound (if not running a Windows game natively on Windows; if Windows executable is not found on Windows, returns LaunchError::IoError)

### state

pub trait StateHandler: Send + Sync

- change_state(&mut self, State) - Displays a new State
- revert_state(&mut self) - Replaces current display with the last one (reverts itself if called twice)

pub enum State (implements Display) - used when calling the StateHandler passed by the app:

- StartingExtraction - created by client before extracting is reported
- Extracting(Option<(u64, u64)>) - bytes currently / bytes total
- Validating
- Installing(ValidatedGame)
- Launching(ValidatedGame)
- WaitingForLock(PathBuf)
- Error(AppError)

### error

Contains error types.

pub type Result<T> = Result<T, AppError>

pub enum AppError (implements From<T> for all referenced error types, Display, Debug, Error (same for referenced error types)):

- NoAppDirs
- ExtractError(ExtractError)
- ValidationError(bdgm::BDGMErrors)
- InstallError(InstallError)
- LaunchError(LaunchError)
- RunningError(RunningError)
- ...
- UnsupportedOnThisPlatform(UnsupportedOperation) - displays as "self.0 is not supported on this platform." - prefer over UnsupportedOperation if it is platform-related.
- UnsupportedOperation(UnsupportedOperation) - displays as "self.0 is not supported."
- Other(String)

pub enum ExtractError:

- IoError(std::io::Error)
- ...

pub enum InstallError:

- IoError(std::io::Error)
- ...

pub enum LaunchError:

- RuntimeNotFound(PathBuf)
- IoError(std::io::Error)
- PortTaken(u16)
- ...

pub enum RunningError:

- IoError(std::io::Error) - returned when a launched game exits with an error

pub enum UnsupportedOperation:

- ReadingRawDiscs
