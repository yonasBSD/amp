use crate::commands::{self, application, Result};
use crate::errors::*;
use crate::input::Key;
use crate::models::application::modes::open::DisplayablePath;
use crate::models::application::modes::{PopSearchToken, SearchSelectMode};
use crate::models::application::{Application, Mode, ModeKey};

pub fn accept(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mode) => {
            let selection = mode.selection().ok_or("No command selected")?;

            // Run the selected command.
            (selection.command)(app)?;
        }
        Mode::Open(ref mut mode) => {
            if mode.selection().is_none() {
                bail!("No buffer selected");
            }

            for DisplayablePath(path) in mode.selections() {
                let syntax_definition = app
                    .preferences
                    .borrow()
                    .syntax_definition_name(path)
                    .and_then(|name| app.workspace.syntax_set.find_syntax_by_name(&name).cloned());

                app.workspace
                    .open_buffer(path)
                    .chain_err(|| "Couldn't open a buffer for the specified path.")?;

                let buffer = app.workspace.current_buffer.as_mut().unwrap();

                // Only override the default syntax definition if the user provided
                // a valid one in their preferences.
                if syntax_definition.is_some() {
                    buffer.syntax_definition = syntax_definition;
                }

                app.view.initialize_buffer(buffer)?;
            }
        }
        Mode::Theme(ref mut mode) => {
            let theme_key = mode.selection().ok_or("No theme selected")?;
            app.preferences.borrow_mut().set_theme(theme_key.as_str());
        }
        Mode::SymbolJump(ref mut mode) => {
            let buffer = app
                .workspace
                .current_buffer
                .as_mut()
                .ok_or(BUFFER_MISSING)?;
            let position = mode
                .selection()
                .ok_or("Couldn't find a position for the selected symbol")?
                .position;

            if !buffer.cursor.move_to(position) {
                bail!("Couldn't move to the selected symbol's position");
            }
        }
        Mode::Syntax(ref mut mode) => {
            let name = mode.selection().ok_or("No syntax selected")?;
            let syntax = app.workspace.syntax_set.find_syntax_by_name(name).cloned();
            let buffer = app
                .workspace
                .current_buffer
                .as_mut()
                .ok_or(BUFFER_MISSING)?;
            buffer.syntax_definition = syntax;
        }
        _ => bail!("Can't accept selection outside of search select mode."),
    }

    app.switch_to(ModeKey::Normal);
    commands::view::scroll_cursor_to_center(app).ok();

    Ok(())
}

pub fn search(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.search(),
        Mode::Open(ref mut mode) => mode.search(),
        Mode::Theme(ref mut mode) => mode.search(),
        Mode::SymbolJump(ref mut mode) => mode.search(),
        Mode::Syntax(ref mut mode) => mode.search(),
        _ => bail!("Can't search outside of search select mode."),
    };

    Ok(())
}

pub fn select_next(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.select_next(),
        Mode::Open(ref mut mode) => mode.select_next(),
        Mode::Theme(ref mut mode) => mode.select_next(),
        Mode::SymbolJump(ref mut mode) => mode.select_next(),
        Mode::Syntax(ref mut mode) => mode.select_next(),
        _ => bail!("Can't change selection outside of search select mode."),
    }

    Ok(())
}

pub fn select_previous(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.select_previous(),
        Mode::Open(ref mut mode) => mode.select_previous(),
        Mode::Theme(ref mut mode) => mode.select_previous(),
        Mode::SymbolJump(ref mut mode) => mode.select_previous(),
        Mode::Syntax(ref mut mode) => mode.select_previous(),
        _ => bail!("Can't change selection outside of search select mode."),
    }

    Ok(())
}

pub fn enable_insert(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.set_insert_mode(true),
        Mode::Open(ref mut mode) => mode.set_insert_mode(true),
        Mode::Theme(ref mut mode) => mode.set_insert_mode(true),
        Mode::SymbolJump(ref mut mode) => mode.set_insert_mode(true),
        Mode::Syntax(ref mut mode) => mode.set_insert_mode(true),
        _ => bail!("Can't change search insert state outside of search select mode"),
    }

    Ok(())
}

pub fn disable_insert(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.set_insert_mode(false),
        Mode::Open(ref mut mode) => mode.set_insert_mode(false),
        Mode::Theme(ref mut mode) => mode.set_insert_mode(false),
        Mode::SymbolJump(ref mut mode) => mode.set_insert_mode(false),
        Mode::Syntax(ref mut mode) => mode.set_insert_mode(false),
        _ => bail!("Can't change search insert state outside of search select mode"),
    }

    Ok(())
}

pub fn push_search_char(app: &mut Application) -> Result {
    if let Some(Key::Char(c)) = *app.view.last_key() {
        match app.mode {
            Mode::Command(ref mut mode) => mode.push_search_char(c),
            Mode::Open(ref mut mode) => mode.push_search_char(c),
            Mode::Theme(ref mut mode) => mode.push_search_char(c),
            Mode::SymbolJump(ref mut mode) => mode.push_search_char(c),
            Mode::Syntax(ref mut mode) => mode.push_search_char(c),
            _ => bail!("Can't push search character outside of search select mode"),
        }
    }

    // Re-run the search.
    search(app)
}

pub fn pop_search_token(app: &mut Application) -> Result {
    match app.mode {
        Mode::Command(ref mut mode) => mode.pop_search_token(),
        Mode::Open(ref mut mode) => mode.pop_search_token(),
        Mode::Theme(ref mut mode) => mode.pop_search_token(),
        Mode::SymbolJump(ref mut mode) => mode.pop_search_token(),
        Mode::Syntax(ref mut mode) => mode.pop_search_token(),
        _ => bail!("Can't pop search token outside of search select mode"),
    }

    search(app)?;
    Ok(())
}

pub fn step_back(app: &mut Application) -> Result {
    let selection_available = match app.mode {
        Mode::Command(ref mut mode) => mode.results().count() > 0 && !mode.query().is_empty(),
        Mode::Open(ref mut mode) => mode.results().count() > 0 && !mode.query().is_empty(),
        Mode::Theme(ref mut mode) => mode.results().count() > 0 && !mode.query().is_empty(),
        Mode::SymbolJump(ref mut mode) => mode.results().count() > 0 && !mode.query().is_empty(),
        Mode::Syntax(ref mut mode) => mode.results().count() > 0 && !mode.query().is_empty(),
        _ => bail!("Can't pop search token outside of search select mode"),
    };

    if selection_available {
        disable_insert(app)
    } else {
        application::switch_to_normal_mode(app)
    }
}
