use crate::commands::{self, Result};
use crate::errors::*;
use crate::input::KeyMap;
use crate::models::application::{Application, Mode, ModeKey};
use crate::util;
use scribe::Buffer;
use std::fs::{read_to_string, remove_file, File};
use std::path::PathBuf;

pub fn handle_input(app: &mut Application) -> Result {
    // Listen for and respond to user input.
    let commands = app.view.last_key().as_ref().and_then(|key| {
        app.mode_str()
            .and_then(|mode| app.preferences.borrow().keymap().commands_for(mode, key))
    });

    if let Some(coms) = commands {
        // Run all commands, stopping at the first error encountered, if any.
        for com in coms {
            debug_log!("[application]: running command");

            com(app)?;

            debug_log!("[application]: command completed successfully");
        }
    }

    Ok(())
}

pub fn switch_to_normal_mode(app: &mut Application) -> Result {
    let _ = commands::buffer::end_command_group(app);
    app.switch_to(ModeKey::Normal);

    Ok(())
}

pub fn switch_to_insert_mode(app: &mut Application) -> Result {
    if app.workspace.current_buffer.is_some() {
        commands::buffer::start_command_group(app)?;
        app.switch_to(ModeKey::Insert);
        commands::view::scroll_to_cursor(app)?;
    } else {
        bail!(BUFFER_MISSING);
    }

    Ok(())
}

pub fn switch_to_jump_mode(app: &mut Application) -> Result {
    let line = app
        .workspace
        .current_buffer
        .as_ref()
        .ok_or(BUFFER_MISSING)?
        .cursor
        .line;

    app.switch_to(ModeKey::Jump);
    if let Mode::Jump(ref mut mode) = app.mode {
        mode.reset(line)
    }

    Ok(())
}

pub fn switch_to_second_stage_jump_mode(app: &mut Application) -> Result {
    switch_to_jump_mode(app)?;
    if let Mode::Jump(ref mut mode) = app.mode {
        mode.first_phase = false;
    } else {
        bail!("Cannot enter second stage jump mode from other modes.");
    };

    Ok(())
}

pub fn switch_to_line_jump_mode(app: &mut Application) -> Result {
    if app.workspace.current_buffer.is_some() {
        app.switch_to(ModeKey::LineJump);
        if let Mode::LineJump(ref mut mode) = app.mode {
            mode.reset();
        }
    } else {
        bail!(BUFFER_MISSING);
    }

    Ok(())
}

pub fn switch_to_open_mode(app: &mut Application) -> Result {
    let exclusions = app.preferences.borrow().open_mode_exclusions()?;
    let config = app.preferences.borrow().search_select_config();

    app.switch_to(ModeKey::Open);
    if let Mode::Open(ref mut mode) = app.mode {
        mode.reset(
            &mut app.workspace,
            exclusions,
            app.event_channel.clone(),
            config,
        )?;
    }

    commands::search_select::search(app)?;

    Ok(())
}

pub fn switch_to_command_mode(app: &mut Application) -> Result {
    let config = app.preferences.borrow().search_select_config();

    app.switch_to(ModeKey::Command);
    if let Mode::Command(ref mut mode) = app.mode {
        mode.reset(config)
    }

    commands::search_select::search(app)?;

    Ok(())
}

pub fn switch_to_symbol_jump_mode(app: &mut Application) -> Result {
    app.switch_to(ModeKey::SymbolJump);

    let token_set = app
        .workspace
        .current_buffer_tokens()
        .chain_err(|| BUFFER_TOKENS_FAILED)?;
    let config = app.preferences.borrow().search_select_config();

    match app.mode {
        Mode::SymbolJump(ref mut mode) => mode.reset(&token_set, config),
        _ => Ok(()),
    }?;

    commands::search_select::search(app)?;

    Ok(())
}

pub fn switch_to_theme_mode(app: &mut Application) -> Result {
    let themes = app
        .view
        .theme_set
        .themes
        .keys()
        .map(|k| k.to_string())
        .collect();
    let config = app.preferences.borrow().search_select_config();

    app.switch_to(ModeKey::Theme);
    if let Mode::Theme(ref mut mode) = app.mode {
        mode.reset(themes, config)
    }

    commands::search_select::search(app)?;

    Ok(())
}

pub fn switch_to_select_mode(app: &mut Application) -> Result {
    let position = *app
        .workspace
        .current_buffer
        .as_ref()
        .ok_or(BUFFER_MISSING)?
        .cursor;

    app.switch_to(ModeKey::Select);
    if let Mode::Select(ref mut mode) = app.mode {
        mode.reset(position);
    }

    Ok(())
}

pub fn switch_to_select_line_mode(app: &mut Application) -> Result {
    let line = app
        .workspace
        .current_buffer
        .as_ref()
        .ok_or(BUFFER_MISSING)?
        .cursor
        .line;

    app.switch_to(ModeKey::SelectLine);
    if let Mode::SelectLine(ref mut mode) = app.mode {
        mode.reset(line);
    }

    Ok(())
}

pub fn switch_to_search_mode(app: &mut Application) -> Result {
    if app.workspace.current_buffer.is_some() {
        app.switch_to(ModeKey::Search);
    } else {
        bail!(BUFFER_MISSING);
    }

    Ok(())
}

pub fn switch_to_path_mode(app: &mut Application) -> Result {
    let path = app
        .workspace
        .current_buffer
        .as_ref()
        .ok_or(BUFFER_MISSING)?
        .path
        .as_ref()
        .map(|p|
            // The buffer has a path; use it.
            p.to_string_lossy().into_owned())
        .unwrap_or_else(||
            // Default to the workspace directory.
            format!("{}/", app.workspace.path.to_string_lossy()));

    app.switch_to(ModeKey::Path);
    if let Mode::Path(ref mut mode) = app.mode {
        mode.reset(path)
    }

    Ok(())
}

pub fn switch_to_syntax_mode(app: &mut Application) -> Result {
    // We'll need a buffer to apply the syntax,
    // so check before entering syntax mode.
    let _ = app
        .workspace
        .current_buffer
        .as_ref()
        .ok_or("Switching syntaxes requires an open buffer")?;

    app.switch_to(ModeKey::Syntax);
    let config = app.preferences.borrow().search_select_config();
    let syntaxes = app
        .workspace
        .syntax_set
        .syntaxes()
        .iter()
        .map(|syntax| syntax.name.clone())
        .collect();
    if let Mode::Syntax(ref mut mode) = app.mode {
        mode.reset(syntaxes, config)
    }

    commands::search_select::search(app)?;

    Ok(())
}

pub fn run_file_manager(app: &mut Application) -> Result {
    let mut command = app
        .preferences
        .borrow()
        .file_manager_command()
        .chain_err(|| "No file manager configured.")?;
    let path = app
        .preferences
        .borrow()
        .file_manager_tmp_file_path()
        .to_path_buf();

    // Some FMs don't create temp files if a selection isn't made.
    // Creating one normalizes expectations after executing it.
    File::create(&path).chain_err(|| "Failed to create file manager temp file")?;

    // Run FM
    app.view.replace(&mut command)?;

    // Read/clean up temp file
    let file_manager_selections =
        read_to_string(&path).chain_err(|| "Failed to read file manager temp file")?;
    remove_file(&path).chain_err(|| "Failed to clean up file manager temp file")?;

    // Open selected buffers
    for selection in file_manager_selections.lines() {
        let path = PathBuf::from(selection);
        util::open_buffer(&path, app)?
    }

    Ok(())
}

pub fn display_default_keymap(app: &mut Application) -> Result {
    commands::workspace::new_buffer(app)?;

    if let Some(buffer) = app.workspace.current_buffer.as_mut() {
        buffer.insert(KeyMap::default_data());
    }

    Ok(())
}

pub fn display_quick_start_guide(app: &mut Application) -> Result {
    commands::workspace::new_buffer(app)?;

    if let Some(buffer) = app.workspace.current_buffer.as_mut() {
        buffer.insert(include_str!("../../documentation/quick_start_guide"));
    }

    Ok(())
}

pub fn display_available_commands(app: &mut Application) -> Result {
    commands::workspace::new_buffer(app)?;

    if let Some(buffer) = app.workspace.current_buffer.as_mut() {
        let command_hash = commands::hash_map();
        let mut command_keys = command_hash.keys().collect::<Vec<&&str>>();
        command_keys.sort();
        command_keys.reverse();
        for key in command_keys {
            buffer.insert(format!("{key}\n"));
        }
    }

    Ok(())
}

pub fn display_last_error(app: &mut Application) -> Result {
    let error = app.error.take().ok_or("No error to display")?;
    let scope_display_buffer = {
        let mut error_buffer = Buffer::new();
        // Add the proximate/contextual error.
        error_buffer.insert(format!("{error}\n"));

        // Print the chain of other errors that led to the proximate error.
        for err in error.iter().skip(1) {
            error_buffer.insert(format!("caused by: {err}"));
        }

        error_buffer
    };
    util::add_buffer(scope_display_buffer, app)
}

pub fn suspend(app: &mut Application) -> Result {
    app.view.suspend();

    Ok(())
}

pub fn exit(app: &mut Application) -> Result {
    app.switch_to(ModeKey::Exit);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::models::application::{Mode, Preferences};
    use crate::models::Application;
    use scribe::Buffer;
    use serial_test::serial;
    use std::env;
    use std::fs::read_to_string;
    use std::path::PathBuf;
    use yaml_rust::yaml::YamlLoader;

    #[test]
    fn display_available_commands_creates_a_new_buffer() {
        let mut app = Application::new(&Vec::new()).unwrap();
        super::display_available_commands(&mut app).unwrap();

        assert!(app.workspace.current_buffer.is_some());
    }

    #[test]
    fn display_available_commands_populates_new_buffer_with_alphabetic_command_names() {
        let mut app = Application::new(&Vec::new()).unwrap();
        super::display_available_commands(&mut app).unwrap();

        let buffer_data = app.workspace.current_buffer.as_ref().unwrap().data();
        let mut lines = buffer_data.lines();
        assert_eq!(
            lines.nth(0),
            Some("application::display_available_commands")
        );
        assert_eq!(lines.last(), Some("workspace::next_buffer"));
    }

    #[test]
    fn switch_to_path_mode_inserts_workspace_directory_as_default() {
        let mut app = Application::new(&Vec::new()).unwrap();

        let buffer = Buffer::new();
        app.workspace.add_buffer(buffer);

        super::switch_to_path_mode(&mut app).unwrap();
        let mode_input = match app.mode {
            Mode::Path(ref mode) => Some(mode.input.clone()),
            _ => None,
        };
        assert_eq!(
            mode_input,
            Some(format!("{}/", app.workspace.path.to_string_lossy()))
        );
    }

    #[test]
    fn switch_to_path_mode_inserts_buffer_path_if_one_exists() {
        let mut app = Application::new(&Vec::new()).unwrap();

        let mut buffer = Buffer::new();
        let absolute_path = format!("{}/test", app.workspace.path.to_string_lossy());
        buffer.path = Some(PathBuf::from(absolute_path.clone()));
        app.workspace.add_buffer(buffer);

        super::switch_to_path_mode(&mut app).unwrap();
        let mode_input = match app.mode {
            Mode::Path(ref mode) => Some(mode.input.clone()),
            _ => None,
        };
        assert_eq!(mode_input, Some(absolute_path));
    }

    #[test]
    fn switch_to_path_mode_raises_error_if_no_buffer_is_open() {
        let mut app = Application::new(&Vec::new()).unwrap();

        // The application type picks up on test run
        // arguments and will open empty buffers for each.
        app.workspace.close_current_buffer();

        assert!(super::switch_to_path_mode(&mut app).is_err());
    }

    #[test]
    #[serial]
    fn run_file_manager_executes_command_and_opens_path_written_to_tmp_file() {
        let dir = env::current_dir().unwrap();
        let cwd = dir.display();

        // Set up the application with a mock command that simulates a file
        // manager by writing a file selection to the tmp file path.
        let mut app = Application::new(&Vec::new()).unwrap();
        let data = YamlLoader::load_from_str(&format!(
            "
                file_manager:
                  command: sh
                  options: ['-c', 'printf {cwd}/Cargo.toml > {}']
            ",
            "${tmp_file}",
        ))
        .unwrap();
        let preferences = Preferences::new(data.into_iter().nth(0));
        app.preferences.replace(preferences);

        super::run_file_manager(&mut app).unwrap();

        assert_eq!(
            app.workspace.current_buffer.as_ref().unwrap().data(),
            read_to_string("Cargo.toml").unwrap()
        );
    }

    #[test]
    #[serial]
    fn run_file_manager_handles_multiple_paths_written_to_tmp_file() {
        let dir = env::current_dir().unwrap();
        let cwd = dir.display();

        // Set up the application with a mock command that simulates a file
        // manager by writing a file selection to the tmp file path.
        let mut app = Application::new(&Vec::new()).unwrap();
        let data = YamlLoader::load_from_str(&format!(
            "
                file_manager:
                  command: sh
                  options: ['-c', 'printf \"{cwd}/Cargo.toml\\n{cwd}/Cargo.lock\" > {}']
            ",
            "${tmp_file}",
        ))
        .unwrap();
        let preferences = Preferences::new(data.into_iter().nth(0));
        app.preferences.replace(preferences);

        super::run_file_manager(&mut app).unwrap();

        assert_eq!(
            app.workspace.current_buffer.as_ref().unwrap().data(),
            read_to_string("Cargo.lock").unwrap()
        );
        app.workspace.next_buffer();
        assert_eq!(
            app.workspace.current_buffer.as_ref().unwrap().data(),
            read_to_string("Cargo.toml").unwrap()
        );
    }
}
