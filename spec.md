# BDGM Blu-Ray Disc Game specification
BDGM is a UDF specification for storing a cross-platform game.
## Folder structure
- BDGM/
    - DISC.BDGM
    - APP/
        - executable, runtime dependencies, resources
- The BDGM application root is `/BDGM/`.
- Paths use the `/` separator and are case-sensitive.
- Names of BDGM-defined files and directories must be UTF-8.
- The BDGM file system must be considered read-only by the game and player.

## DISC.BDGM
DISC.BDGM is a property file in the BDGM Properties Format (BPF):
- BPF is a text format in UTF-8 encoding.
- First line must be a format/version specifier: `BDGM/<version>`
- The rest of the file are key-value pairs separated with the first `=`. Spaces before `=` are forbidden. Whitespace in the value are not ignored.
- Leading and trailing whitespaces of the line are forbidden.
- Empty values are forbidden; if a key is optional, omit it instead of setting it to empty.
- Duplicate keys are forbidden.
- Keys cannot include `=` (because it would be used to separate key and value).
- Empty lines are ignored.
- Lines without `=` are ignored.
- Lines starting with `#` are comments.
- Unknown keys are ignored.
- Discs with any missing mandatory key must be rejected.

### BDGM/1.0
- Keys:
    - `name`: The name of the stored game, spaces included. Mandatory.
    - `id`: The unique identifier of the stored game in the period-separated kebab-case reverse domain notation. Mandatory.
        - Uses lowercase ASCII characters, digits and hyphens. Spaces are not permitted.
        - This ID must not change between versions.
        - Separating different components in a larger piece of work within the ID is allowed, such as: `com.example.example-game.main-game`
        - Example: `com.example.my-game`.
    - `version`: The version of the stored game, in a text representation. Semantic versioning with major, minor and patch versions is suggested but not required. Mandatory.
    - `runtime`: The runtime used to run the stored game. This is one of `java`, `dotnet`, `python`, `windows`. Other options may be added by future versions of the spec. Mandatory.
    - `runtime_version`: The version of the runtime used to run the stored game (see Runtimes). Optional for `windows`, otherwise mandatory.
    - `executable`: The path to the executable to run using the runtime. This is a path to the file relative to the BDGM/APP directory. Mandatory.
        - Absolute paths and `.` or `..` path components are forbidden. An initial `/` is forbidden.
        - The path must resolve to a regular file within `/BDGM/APP/` and must not resolve outside the directory.
        - Example: `executable=game.jar`
    - `args`: Arguments passed to the executable in a JSON array. If running through a runtime, these are often passed after ` -- `, depending on the runtime. Optional.
    - `runtime_args`: Arguments passed to the runtime itself in a JSON array. For `windows`, these are resolved only if running on another operating system than Windows. Optional.

`args` and `runtime_args` must be valid JSON arrays containing only string elements. Each element represents exactly one argument. No shell parsing is performed.

## Executable
The executable is a file runnable by the selected runtime (or a `.exe` file in the case of `windows`).
It must be located in `APP/`.
It should not be in a subdirectory of `APP/`. If that is impossible, it must not be a wrapper. Instead, set the `executable` field to the actual path of the executable.
The executable must function without internet access (it must not crash because of no internet access, it may show a connection error screen instead).

## Runtimes
- `java` - uses a JRE of the specified major version to run the .jar file (or .class files in special cases). `runtime_version` means the Java SE version.
    - The executable must be a runtime-dependent architecture-agnostic build.
- `dotnet` - uses a .NET runtime of the specified major version to run the .dll file. `runtime_version` specifies the version of the *.NET runtime*, not the C# version.
    - The executable must be a runtime-dependent architecture-agnostic build.
- `python` - uses a Python interpreter of the specified version to run the python file. `runtime_version` specifies the Python *major and minor* version separated with a period. (e.g. `runtime_version=3.14`)
    - The executable must be a `.py` or `.pyz` file.
    - The player must run `.py` and `.pyz` files through the filesystem path, not as a module.
    - The `.pyz` file must include all Python code and pure Python dependencies.
    - Native dependencies (such as `pygame`) must be included in the `APP/` directory for all supported platforms. These dependencies must be importable by normal Python import or any executable-defined loading mechanism.
    - If supplying dependencies with the above restrictions is unfeasible, you may use a bundler such as `pyinstaller` to make a self-contained x86_64 Windows binary and select the `windows` runtime and use a compatibility layer.
- `windows` - On Windows, executes the .exe file directly. Otherwise, uses Wine or a compatible Wine implementation. If specified, `runtime_version` is the minimum Windows compatibility target compatible with the game.
    - The executable must be built for x86_64.
    - On other architectures and OSes, a compatibility layer will be used if possible.
OS-specific or architecture-specific files should be included for all supported systems and OS-agnostic solutions are preferred.
The game should access resources using a path relative to the running bundle/executable instead of relying on the current working directory.

## Player
The player is an application installed on the user's operating system that reads the BDGM disc or image and runs the game. It may bundle common versions of runtimes.
Whether bundled runtimes are used instead of user-installed ones depends on the player and/or user preference.
The player must only use runtimes compatible with the major version specified in `DISC.BDGM`. A newer runtime may only be used if known to be compatible with the specified version.
The player must create and provide a writable data and cache directory for the game. These must be different (though they may be nested).
Each game's cache and data directories must be different. Different games must be identified via `id`.
Different versions of games with the same `id` must use the same data directory.
Cache directory may be emptied or replaced before running a game with a different version.
If possible by the operating system, the player may provide write redirection to the data directory for games that write data into the installation or current working directory.

The player must:
1. Locate `/BDGM/DISC.BDGM`, and reject disc otherwise.
2. Parse the contents of `DISC.BDGM`.
3. Validate the `executable` path based on rules defined above.
4. Verify the executable referenced in `DISC.BDGM` exists.
5. Locate an appropriate runtime, either installed on the user's OS (via `PATH`) or bundled with the player. If none are present, the player may download one or prompt the user to install one.
    If the user rejects installation of a missing runtime, or one cannot be downloaded, the disc must be rejected.
6. Set the CWD to the executable's directory.
7. Set up directory redirection for games placing data in executable directory if possible and supported by the player.
8. Set system envvars to the directories created by the player and set BDGM environment variables.
9. Launch the game with arguments specified in `DISC.BDGM`.

### Environment variables
These environment variables must be provided by the players. Games may read these variables to get assets or save data.
- `BDGM_DATA`: The data dir created by the player. This must be persistent between game executions.
- `BDGM_CACHE`: The cache dir created by the player. This may be deleted at any time while the game is not running. The player must not delete it while the game is running.
- `BDGM_APP`: The path to the `APP` directory, not the executable's directory.
- `BDGM_DISC`: The path to the disc root (`/`, not `/BDGM/`).
- `BDGM_VERSION`: The version of the BDGM specification used by the disc. (e.g. `1.0`)
- `WINEPREFIX`: Only if the game is `windows`; must be set to a writable location.
Games must not assume these paths are on the same storage device.
