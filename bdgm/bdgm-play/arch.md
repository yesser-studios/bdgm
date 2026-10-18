# BDGM Player arch

This document describes the design of bdgm-play.

bdgm-play has:
- main.rs - calls app::run
- lib.rs - refs every module in the app.

definitions:
- image: a .bin, .udf or other binary image of a BDGM disc (UDF 2.50/2.01)
- mounted disc: a disc already read by the system and mounted to a directory (or an unbuilt source dir)
- raw disc: a disc not read by the system, referenced by its path (Windows: '\\.\X:') that should be extracted by the player.
- atomically write: Write into file/directory named "*.part" and rename after writing.

**Only items defined in this document may be exported outside of their module. Helper functions added to supplement these should be declared in the same module, and may not be pub.**

## app
- run() Parses args (using clap) and calls run_gui or run_cli accordingly. If LOCATION argument is not specified, and --raw-disc or --image flags are not enabled, use gui.  
If LOCATION is specified, use cli.  
If LOCATION is not specified and either --raw-disc or --image are enabled, display an error stating that no location was specified and halt.

## args
Contains the Args struct (parseable via clap):
- location: Option<PathBuf> - determines the location of the source. Option because GUI doesn't read this.
- image: bool - flag - whether there is an image at the location.
- raw-disc: bool - flag - ONLY ON WINDOWS - whether there is a raw disc at the location.
- runtime: Option<PathBuf> - flag - specifies the location of the runtime to use to run the app, instead of the default one. CLI and GUI read this.

Args should not go any further than to run_gui or run_cli.

## cli

run_cli(args) - Parses args into a LaunchRequest and launches it via launch_request.

CliStateHandler (implements StateHandler)
- change_state - println!s State
- revert_state - no-op (previous state is already in logs)

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
- run_gui(args) Launches iced window and sets initial state based on the runtime specified in args.

### state
Contains the application's current state in multiple structs.

AppState:
- runtime: Option<PathBuf> - from args, replaces default runtime.
- phase: Phase - the phase the app is currently in.
- status: State - the current status of core.

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
- _tempdir: Option<tempfile::TempDir> - holds a temp dir until installed, to delete when dropped.
- validate(&self, &dyn StateHandler) -> Result<ValidatedSource> - Calls functions from validate and creates a ValidatedSource from the returned bdgm::ValidatedGame. Emits Validating to StateHandler.

struct ValidatedSource
- game: bdgm::ValidatedGame
- path: PathBuf
- _tempdir: Option<tempfile::TempDir>
- install(Self, &AppDirs, &dyn StateHandler) -> Result<InstalledGame> - calls functions from install and creates an InstalledGame from the returned GameDirs. Emits Installing to StateHandler.

struct InstalledGame
- game: bdgm::ValidatedGame
- game_dirs: GameDirs - the game's installation and data dir

### resolve
This module resolves a Source. If the game's version's install dir already exists and executable exists inside it, skip extraction and return InstalledGame.  
resolve_image/raw_disc/mounted parse BDGM/DISC.BDGM into bdgm::Game and validate to bdgm::ValidatedGame (using get_*_manifest from the related module); if Ok and version is already installed, create and return Installed, otherwise extract.  
If manifest could not be read or parsed, extract regardless of whether it is already installed. If manifest could not be validated return a ValidationError.

resolve(Source, &AppDirs, &dyn StateHandler) -> Result<ResolvedSource> - uses functions below to extract a Source and returns a ResolvedSource. Source is consumed. Emits Extracting to StateHandler.

resolve_image(&Path, &AppDirs, &dyn StateHandler) -> Result<ResolvedSource> - extracts an Image.

resolve_raw_disc(&Path, &AppDirs, &dyn StateHandler) -> Result<ResolvedSource> - Returns UnsupportedOnThisPlatform outside Windows.

resolve_mounted(&Path, &AppDirs, &dyn StateHandler) -> Result<ResolvedSource> - Points ExtractedSource at itself (since the disc is already mounted on the filesystem, there's no other fs to extract from)

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

extract(&UdfVolume<impl Read + Seek>, &dyn StateHandler) -> Result<ExtractedSource> - uses extract_dir.

extract_dir(&UdfVolume<impl Read + Seek>, &UdfDir, &Path, current_size: &mut u64, total_size: &u64, &dyn StateHandler) -> Result<()> - recursive  
Skip all elements escaping the UDF volume - reject all UdfDirs with paths that are not single-component, not absolute, and not current dir or parent dir.

#### mounted

get_mounted_manifest(&Path) -> Result<Option<bdgm::Game>>

### validate
Validates an ExtractedSource.

validate(source: ExtractedSource) -> Result<bdgm::ValidatedGame> - opens and validates the manifest from the extracted source and checks if executable exists.

executable_exists(&ValidatedGame, app_path : &PathBuf) -> bool - Checks if the executable exists within the specified BDGM/APP dir

### install
Installs a validated extracted source.

install(source: ValidatedSource, &AppDirs, &dyn StateHandler) -> Result<GameDirs> - Creates game dirs, then installs the game into the install dir specified in the tree under dirs and returns the game's dirs.  
Wait for lock (install_dir.lock), then check: if install dir and executable exists, skip installation. If install dir exists but executable doesn't, then remove and overwrite install dir.  
Should atomically write the installation directory.  
If a stale install_dir.part exists after locking before installation, that means an installation was interrupted; remove existing part and proceed as if it didn't exist.

check_installed(id: &str, version: &str, &AppDirs, &dyn StateHandler) -> Result<bool> - Checks whether the installation directory of the specified version of the ID exists.  
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

get_app_dirs() -> Result<AppDirs> - gets app dir paths

create_game_dirs(&GameDirs) -> io::Result<()> - creates game data, cache and parent of install_dir specified in the tree under dirs if they don't exist.

### fs
Helpers for manipulating files and directories.

acquire_lock(&Path, blocking: bool, Option<&dyn StateHandler>) -> std::io::Result<File> - opens (path).lock and locks it; if blocking = true, tries lock and if would block, sets state to WaitingForLock(lock file path) (if StateHandler is Some) and then resets back.

open_file(&Path) -> io::Result<File> - simple wrapper around a file options builder for writable file open

open_part_file(&Path) -> io::Result<File> - Opens (path).part

truncate(&mut File) -> io::Result<()> - clears file

### launch
This module launches a LaunchRequest.

pub launch_request(LaunchRequest, on_state: &dyn StateHandler) -> Result<(ValidatedGame, LaunchHandle)> - resolves, validates, installs (if not already installed) and runs a launch request. This is called by the client.

run_game(&InstalledGame, runtime: Option<&Path>, &dyn StateHandler) -> Result<LaunchHandle> - either runs HTML server or creates game data and cache dirs and builds and runs command.

#### handle
pub enum LaunchHandle (implements Clone) - wraps GameProcess, Arc<HtmlServer>:
- async kill(self) -> io::Result<Option<ExitStatus>> - kills LaunchHandle and, if HtmlServer, returns Ok(ExitStatus::ok (code 0)); if GameProcess, calls wait on it.
- wait will only be called by matching and calling wait on GameProcess (HtmlServer would hang indefinitely)

#### server
pub struct HtmlServer (implements Drop - aborts on drop) - contains server information and kills it when dropped:
- port: u16
- entry_point: String - the URI to the entry point of the server
- abort: tokio::task::AbortHandle
- start(dir: &Path, port: u16) -> Result<Self> - starts a new server at 127.0.0.1:port. If the port is already taken, error with PortTaken.
- get_url(&self) -> &str
- open(&self) - opens the entry point in a browser
- kill(&self) - aborts the AbortHandle

unique_port(id: &str, &dyn StateHandler) -> Result<u16> - gets persisted unique port for this game ID (stored in data_dir/ports.json), or creates one (no two IDs may share a port)  
ports.json must be written atomically and wait for lock.  
Creates ports.json parent dir if it doesn't exist.

start_server(&GameDirs, entry_point: String, &dyn StateHandler) -> Result<HtmlServer> - gets paths and ID from GameDirs, starts a server for install_dir and opens in browser, creating a unique port in the process  
The executable path is the path to the index endpoint from the root of APP (installed into install_dir)

#### process
pub struct GameProcess(Arc<tokio::Mutex<tokio::process::Child>>) (implements Clone)
- kill(&self) - starts killing process (doesn't wait)
- wait(self) -> io::Result<Option<ExitStatus>> - waits for process to end and returns its exit status. Instead of awaiting it periodically acquires lock and checks if process exited.
- kill_wait(self) -> io::Result<Option<ExitStatus>> - kills process and waits

get_envvars(&InstalledGame) -> HashMap<&'static str, String> - gets environment vars as specified in the spec. Use InstalledGame.game.bdgm_version for BDGM_VERSION. See the tree under dirs.

build_command(&InstalledGame, runtime: Option<&Path>) -> Result<tokio::process::Command> - calls get_envvars; matches the game's runtime field, uses build_bin_command (calls function executable from GameDirs); the HTML branch should be unreachable as that is checked in run_game

build_bin_command(default_runtime: &Path, executable: &Path, install_dir: &Path, runtime: Option<&Path>, runtime_args: &[&str], args: &[&str], envvars: &HashMap<&'static str, String>) -> Result<tokio::process::Command> - generates a command.
Sets working directory to executable.parent

run_command(tokio::process::Command) -> Result<LaunchHandle> - Child process will be kill on drop; maps io::NotFound to RuntimeNotFound (if not running a Windows game natively on Windows)

### state
pub trait StateHandler: Send + Sync
- change_state(&self, State) - Displays a new State
- revert_state(&self) - Replaces current display with the last one (reverts itself if called twice)

pub enum State (implements Display) - used when calling the StateHandler passed by the app:
- Extracting(Option<(u64, u64)>) - bytes currently / bytes total
- Validating
- Installing(ValidatedGame)
- Launching(ValidatedGame)
- WaitingForLock(PathBuf)
- Error(AppError)

### error
Contains error types.

pub type Result<T> = Result<T, AppError>

pub enum AppError (implements From<T> for all referenced error types, Display, Debug, Error):
- NoAppDirs
- ExtractError(ExtractError)
- ValidationError(bdgm::BDGMErrors)
- InstallError(InstallError)
- LaunchError(LaunchError)
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

pub enum UnsupportedOperation:
- ReadingRawDiscs
