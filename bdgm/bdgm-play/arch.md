# BDGM Player arch

This document describes the design of bdgm-play.  
bdgm-play has two entry points: main.rs, the sole purpose of which is to call app::run, and lib.rs that refs every module in the app.

defintions:
- image: a .bin, .udf or other binary image of a BDGM disc (UDF 2.50/2.01)
- mounted disc: a disc already read by the system and mounted to a directory (or an unbuilt source dir)
- raw disc: a disc not read by the system, referenced by its path (Windows: '\\.\X:') that should be extracted by the player.

## app
- run() Parses args (using serde) and calls run_gui or run_cli accordingly. If LOCATION argument is not specified, and --raw-disc or --image flags are not enabled, use gui.  
If LOCATION is specified, use cli.  
If LOCATION is not specified and either --raw-disc or --image are enabled, display an error stating that no location was specified and halt.

## args
Contains the Args struct (parseable via serde):
- location: Option<PathBuf> - determines the location of the source. Option because GUI doesn't read this.
- image: bool - flag - whether there is an image at the location.
- raw-disc: bool - flag - ONLY ON WINDOWS - whether there is a raw disc at the location.
- runtime: Option<PathBuf> - flag - specifies the location of the runtime to use to run the app, instead of the default one. CLI and GUI read this.

Args should not go any further than to run_gui or run_cli.

## cli
- run_cli(args) Parses args into a LaunchRequest (see below) and launches it via APIs from core.  
Progress reporting will be done via println or eprintln.

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
- status: Status - the current status of core.

## core
This module includes all "business logic" of bdgm-play, as described below.  
The goal of core is to provide APIs for launching a launch request.

pub use everything marked with pub in this file

### request
This module contains LaunchRequest and related structs.

pub struct LaunchRequest:
- source: Source
- runtime: Option<PathBuf>

pub struct Source:
- path: PathBuf
- kind: SourceKind
- resolve(self, &mut StateHandler) - uses core::resolve to extract a Source and returns a ResolvedSource. Source is consumed. (MountedDisc will resolve directly as no operation is needed)

pub enum SourceKind:
- MountedDisc
- RawDisc (include on non-Windows)
- Image

enum ResolvedSource
- Extracted(ExtractedSource) - an extracted directory from a source, to be installed.
- Validated(ValidatedSource) - a Source that has been extracted and validated, but not installed.
- Installed(InstalledGame) - a Source that has already been installed and does not need to be extracted.

struct ExtractedSource
- path: PathBuf - the location of the extacted dir's root.
- original_path: PathBuf - The path to the original source.
- _tempdir: Option<tempfile::TempDir> - holds a temp dir until installed, to delete when dropped.
- validate(Self) -> Result<ValidatedSource> - calls functions from validate

struct ValidatedSource
- game: bdgm::ValidatedGame
- path: PathBuf
- original_path: PathBuf - The path to the original source.
- _tempdir: Option<tempfile::TempDir>
- install(Self) -> Result<ValidatedGame> - calls functions from install

struct InstalledGame
- game: bdgm::ValidatedGame
- game_dirs: GameDirs - the game's installation and data dir
- original_path: PathBuf - The path to the original source.

### resolve
This module resolves a Source.
- resolve_image(&Path, &mut StateHandler) -> Result<ResolvedSource> - extracts an Image.
- resolve_raw_disc(&Path, &mut StateHandler) -> Result<ResolvedSource> - returns Unsupported error on non-Windows, otherwise checks if disc has already been installed and if not, extracts it. (uses functions from resolve::raw_disc)

#### raw_disc
Contains a RawDiscReader that wraps File and reads it in fixed sectors, and logic to check and extract the disc.

SECTOR_SIZE = 2048

RawDiscReader (implements Seek, Read):
- file: File
- pos: u64,
- len: u64,
- cache_sector: u64,
- cache: [u8; SECTOR_SIZE],
- cache_valid: bool,
- impl From<File> - creates a new reader from a file.
- open(&Path) -> Result<Self> - creates a new reader from a path to the disc. Returns an error if the file could not be opened or if the path is not a disc.

#### udf
open_udf_volume<T: Read + Seek>(T) -> Result<UdfVolume<T>>

get_manifest(&UdfVolume<impl Read + Seek>) -> Result<Option<bdgm::Game>> (not ValidatedGame)

extract(&UdfVolume<impl Read + Seek>, &mut StateHandler) -> Result<ResolvedSource> - uses extract_dir

extract_dir(&UdfVolume<impl Read + Seek>, &UdfDir, &Path, &mut StateHandler) -> Result<()> - recursive

### validate
Validates an ExtractedSource.

validate(source: ExtractedSource) -> Result<bdgm::ValidatedGame> - opens and validates the manifest and checks if executable exists

executable_exists(&ValidatedGame) -> bool

### install
Installs a validated extracted source.

install(source: ValidatedSource, &AppDirs) -> Result<GameDirs> - returns the game's dirs.

create_game_dirs(&GameDirs) -> io::Result<()> - creates game data and cache dirs if they don't exist

### dirs
Constructs directory paths.

struct GameDirs:
- data_dir: PathBuf
- cache_dir: PathBuf
- or, instead of data and cache_dir - app_dirs: &AppDirs
- id: String
- version: String
- executable: PathBuf - relative to install dir
- from(&bdgm::ValidatedGame, &AppDirs) - extracts data_dir and cache_dir from AppDirs (or saves borrow) and gets id, version and executable path from ValidatedGame
- root(&self) -> &Path
- install_dir(&self) -> &Path
- cache(&self) -> &Path
- data(&self) -> &Path
- executable(&self) -> &Path
- id(&self) -> &str

get_manifest_path(&Path) -> PathBuf - gets path to BDGM/DISC.BDGM with a disc root path

get_app_dir_path(&Path) -> PathBuf - gets path to BDGM/APP/ with a disc root path

create_app_dirs() -> Result<AppDirs> - gets app dir paths

### fs
Helpers for manipulating files and directories.

acquire_lock(&Path, blocking: bool, &mut StateHandler) -> Result<File> - opens (path).lock and locks it; if !blocking, tries lock and if would block, sets state to Blocking and then resets back

get_file(&Path) -> io::Result<File> - simple wrapper around a file options builder for writable file open

get_part_file(&Path) -> io::Result<File> - Opens (path).part

truncate(&mut File) -> io::Result<()> - clears file

### launch
This module launches a LaunchRequest.

pub launch_request(LaunchRequest, on_state: &mut StateHandler) -> Result<LaunchHandle> - resolves, validates, installs and runs a launch request.

run_game(&InstalledGame, runtime: Option<&Path>) -> Result<LaunchHandle> - either runs HTML server or builds and runs command.

#### handle
enum LaunchHandle { ... } - wraps GameProceess, HtmlServer:
- wait(self) -> ExitStatus
- kill(self) -> ExitStatus - kills LaunchHandle and calls wait on it.

#### server
struct HtmlServer - contains server information and kills it when dropped:
- port: u16
- entry_point: String - the URI to the entry point of the server
- task: tokio::task::JoinHandle<std::io::Result<()>>
- _shutdown: tokio_util::sync::DropGuard
- start(dir: &Path, port: u16) -> Result<Self>

unique_port(id: &str) -> Result<u16> - gets persisted port for this game ID, or creates one

start_server(&GameDirs, entry_point: String) -> Result<HtmlServer> - gets paths and ID from GameDirs, starts a server and opens in browser, creating a unique port in the process

#### process
struct GameProcess(Option<tokio::process::Child>)
- executable: PathBuf
- drop(&mut self) - kills self.0

get_envvars(&InstalledGame) -> HashMap<&'static str, String>

build_command(&InstalledGame, runtime: Option<&Path>) -> Result<tokio::process::Command> - calls get_envvars

run_command(tokio::process::Command) -> Result<ExitStatus> - returns RuntimeNotFound if exits with FileNotFound (if not Runtime::Windows on Windows platform)

### state
pub struct StateHandler (implements Default with no-op)
- on_state_changed: impl FnMut(State)
- on_revert: impl FnMut() - no-op on CLI (because the text has already been printed)
- current_state: Option<State>
- last_state: Option<State>
- change_state(State) - sets current_state, last_state and calls on_state_changed
- revert_state() - swaps current_state and last_state, calls on_revert
- (if needed) revert_state_to(State) - same as change_state but calls on_revert instead

enum State - used when calling the update_state lambda passed by the app (implements Display):
- Extracting(Option<(u64, u64)>) - bytes currently / bytes total
- Validating
- Installing(ValidatedGame)
- Launching(ValidatedGame)
- WaitingForLock(PathBuf)
- Error(Error)

### error
Contains error types.

type Result<T>: Result<T, AppError>

pub enum AppError (implements From<T> for all referenced error types, Display, Debug, Error):
- NoAppDirs
- ExtractError(ExtractError)
- ValidationError(bdgm::BDGMError)
- InstallError(InstallError)
- LaunchError(LaunchError)
- ...
- Unsupported(String)
- Other(String)

pub enum ExtractError:
- ...

pub enum InstallError:
- IoError(IoError)
- ...

pub enum LaunchError:
- RuntimeNotFound(PathBuf)
- ...

