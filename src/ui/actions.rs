use std::{cell::Cell, rc::Rc};

use crate::{
    app::RnoteApp,
    pens::{shaper, PenStyle},
    strokes::render,
    ui::appwindow::RnoteAppWindow,
    ui::{canvas::Canvas, dialogs},
};
use gtk4::{
    gdk, gio, glib, glib::clone, prelude::*, ArrowType, Grid, PackType, PositionType,
    PrintOperation, PrintOperationAction, Revealer, ScrolledWindow, Separator, Snapshot, Unit,
};

/* Actions follow this principle:
without any state: the activation triggers the callback
with boolean state: They have a boolean parameter, and a boolean state. activating the action can be done with activate_action() with the desired state.
    A state change can also be directly requested with change_action_state( somebool ).
for other stateful actions: They have the same values as their state as their parameters. Activating the action with a parameter is equivalent to changing its state directly
*/
pub fn setup_actions(appwindow: &RnoteAppWindow) {
    let app = appwindow
        .application()
        .unwrap()
        .downcast::<RnoteApp>()
        .unwrap();

    let action_about = gio::SimpleAction::new("about", None);
    let action_clear_sheet = gio::SimpleAction::new("clear-sheet", None);
    let action_open_canvasmenu = gio::SimpleAction::new("open-canvasmenu", None);
    let action_open_appmenu = gio::SimpleAction::new("open-appmenu", None);
    let action_zoom_reset = gio::SimpleAction::new("zoom-reset", None);
    let action_zoom_fit_width = gio::SimpleAction::new("zoom-fit-width", None);
    let action_zoomin = gio::SimpleAction::new("zoom-in", None);
    let action_zoomout = gio::SimpleAction::new("zoom-out", None);
    let action_delete_selection = gio::SimpleAction::new("delete-selection", None);
    let action_duplicate_selection = gio::SimpleAction::new("duplicate-selection", None);
    let action_import_file = gio::SimpleAction::new("import-file", None);
    let action_export_selection_as_svg = gio::SimpleAction::new("export-selection-as-svg", None);
    let action_export_sheet_as_svg = gio::SimpleAction::new("export-sheet-as-svg", None);
    let action_keyboard_shortcuts_dialog = gio::SimpleAction::new("keyboard-shortcuts", None);
    let action_warning =
        gio::SimpleAction::new("warning", Some(&glib::VariantType::new("s").unwrap()));
    let action_error = gio::SimpleAction::new("error", Some(&glib::VariantType::new("s").unwrap()));
    let action_new_sheet = gio::SimpleAction::new("new-sheet", None);
    let action_save_sheet = gio::SimpleAction::new("save-sheet", None);
    let action_save_sheet_as = gio::SimpleAction::new("save-sheet-as", None);
    let action_open_sheet = gio::SimpleAction::new("open-sheet", None);
    let action_open_workspace = gio::SimpleAction::new("open-workspace", None);
    let action_print_sheet = gio::SimpleAction::new("print-sheet", None);
    let action_devel_settings = gio::SimpleAction::new("devel-settings", None);

    let action_tmperaser = gio::SimpleAction::new_stateful(
        "tmperaser",
        Some(&glib::VariantType::new("b").unwrap()),
        &false.to_variant(),
    );
    let action_current_pen = gio::SimpleAction::new_stateful(
        "current-pen",
        Some(&glib::VariantType::new("s").unwrap()),
        &"marker".to_variant(),
    );
    let action_current_shape = gio::SimpleAction::new_stateful(
        "current-shape",
        Some(&glib::VariantType::new("s").unwrap()),
        &"rectangle".to_variant(),
    );
    let action_shaper_drawstyle = gio::SimpleAction::new_stateful(
        "shaper-drawstyle",
        Some(&glib::VariantType::new("s").unwrap()),
        &"smooth".to_variant(),
    );

    let action_devel = appwindow.app_settings().create_action("devel");
    let action_renderer_backend = appwindow.app_settings().create_action("renderer-backend");
    let action_sheet_format_borders = appwindow.app_settings().create_action("format-borders");
    let action_touch_drawing = appwindow.app_settings().create_action("touch-drawing");
    let action_autoexpand_height = appwindow.app_settings().create_action("autoexpand-height");
    let action_righthanded = appwindow.app_settings().create_action("righthanded");
    let action_visual_debug = appwindow.app_settings().create_action("visual-debug");

    // Keyboard shortcuts
    action_keyboard_shortcuts_dialog.connect_activate(
        clone!(@weak appwindow => move |_action_keyboard_shortcuts_dialog, _parameter| {
            dialogs::dialog_keyboard_shortcuts(&appwindow);
        }),
    );
    app.add_action(&action_keyboard_shortcuts_dialog);

    // Warning
    action_warning.connect_activate(
        clone!(@weak appwindow => move |_action_warning, parameter| {
             let warning = parameter.unwrap().get::<String>().unwrap();
            log::warn!("{}", warning);
        }),
    );
    app.add_action(&action_warning);

    // Error
    action_error.connect_activate(clone!(@weak appwindow => move |_action_error, parameter| {
         let error = parameter.unwrap().get::<String>().unwrap();
            log::error!("{}", error);
    }));
    app.add_action(&action_error);

    // Developer mode
    action_devel.connect_state_notify(
        clone!(@weak appwindow, @weak action_devel_settings => move |action_devel| {
            let state = action_devel.state().unwrap().get::<bool>().unwrap();

            action_devel_settings.set_enabled(state);
            appwindow.devel_actions_revealer().set_reveal_child(state);

            if !state {
                appwindow.application().unwrap().change_action_state("visual-debug", &false.to_variant());
            }
        }),
    );
    app.add_action(&action_devel);

    // Developer settings menu
    app.add_action(&action_devel_settings);

    // Visual Debug
    app.add_action(&action_visual_debug);

    // Renderer Backend
    action_renderer_backend.connect_state_notify(clone!(@weak appwindow => move |action_renderer_backend| {
        let state = action_renderer_backend.state().unwrap().get::<String>().expect("wrong type for state of 'action_renderer_backend' must be of type String");
        match state.as_str() {
            "librsvg" => {
                appwindow.canvas().renderer().borrow_mut().backend = render::RendererBackend::Librsvg;
            },
            "resvg" => {
                appwindow.canvas().renderer().borrow_mut().backend = render::RendererBackend::Resvg;
            },
            _ => {
                log::error!("invalid state of action_renderer_backend");
            }
        }

        appwindow.canvas().regenerate_content(true, true);
    }));
    app.add_action(&action_renderer_backend);

    // Touch drawing
    app.add_action(&action_touch_drawing);

    // Current Pen
    action_current_pen.connect_activate(move |action_current_pen, parameter| {
        if action_current_pen.state().unwrap().str().unwrap() != parameter.unwrap().str().unwrap() {
            action_current_pen.change_state(parameter.unwrap());
        }
    });
    action_current_pen.connect_change_state(
        clone!(@weak appwindow => move |action_current_pen, value| {
            action_current_pen.set_state(value.unwrap());

            match action_current_pen.state().unwrap().str().unwrap() {
                "marker" => {
                    appwindow.mainheader().marker_toggle().set_active(true);
                    appwindow.canvas().current_pen().set(PenStyle::Marker);
                    appwindow.penssidebar().sidebar_stack().set_visible_child_name("marker_page");
                },
                "brush" => {
                    appwindow.mainheader().brush_toggle().set_active(true);
                    appwindow.canvas().current_pen().set(PenStyle::Brush);
                    appwindow.penssidebar().sidebar_stack().set_visible_child_name("brush_page");
                },
                "shaper" => {
                    appwindow.mainheader().shaper_toggle().set_active(true);
                    appwindow.canvas().current_pen().set(PenStyle::Shaper);
                    appwindow.penssidebar().sidebar_stack().set_visible_child_name("shaper_page");
                },
                "eraser" => {
                    appwindow.mainheader().eraser_toggle().set_active(true);
                    appwindow.canvas().current_pen().set(PenStyle::Eraser);
                    appwindow.penssidebar().sidebar_stack().set_visible_child_name("eraser_page");
                },
                "selector" => {
                    appwindow.mainheader().selector_toggle().set_active(true);
                    appwindow.canvas().current_pen().set(PenStyle::Selector);
                    appwindow.penssidebar().sidebar_stack().set_visible_child_name("selector_page");
                },
                _ => { log::error!("set invalid state of action `current-pen`")}
            }
        }),
    );
    app.add_action(&action_current_pen);

    // Shaper drawstyle
    action_shaper_drawstyle.connect_activate(move |action_shaper_drawstyle, parameter| {
        if action_shaper_drawstyle.state().unwrap().str().unwrap()
            != parameter.unwrap().str().unwrap()
        {
            action_shaper_drawstyle.change_state(parameter.unwrap());
        }
    });
    action_shaper_drawstyle.connect_change_state(
        clone!(@weak appwindow => move |action_shaper_drawstyle, value| {
            action_shaper_drawstyle.set_state(value.unwrap());

            match action_shaper_drawstyle.state().unwrap().str().unwrap() {
                "smooth" => {
                    appwindow.penssidebar().shaper_page().drawstyle_smooth_toggle().set_active(true);
                    appwindow.canvas().pens().borrow_mut().shaper.drawstyle = shaper::DrawStyle::Smooth;
                },
                "rough" => {
                    appwindow.penssidebar().shaper_page().drawstyle_rough_toggle().set_active(true);
                    appwindow.canvas().pens().borrow_mut().shaper.drawstyle = shaper::DrawStyle::Rough;
                },
                _ => { log::error!("set invalid state of action `shaper-drawstye`")}
            }
        }),
    );
    app.add_action(&action_shaper_drawstyle);

    // Current Shape
    action_current_shape.connect_activate(move |action_current_shape, parameter| {
        if action_current_shape.state().unwrap().str().unwrap() != parameter.unwrap().str().unwrap()
        {
            action_current_shape.change_state(parameter.unwrap());
        }
    });
    action_current_shape.connect_change_state(
        clone!(@weak appwindow => move |action_current_shape, value| {
            action_current_shape.set_state(value.unwrap());

            match action_current_shape.state().unwrap().str().unwrap() {
                "line" => {
                    appwindow.penssidebar().shaper_page().line_toggle().set_active(true);
                    appwindow.canvas().pens().borrow_mut().shaper.current_shape = shaper::CurrentShape::Line;
                    appwindow.penssidebar().shaper_page().fill_revealer().set_reveal_child(false);
                },
                "rectangle" => {
                    appwindow.penssidebar().shaper_page().rectangle_toggle().set_active(true);
                    appwindow.canvas().pens().borrow_mut().shaper.current_shape = shaper::CurrentShape::Rectangle;
                    appwindow.penssidebar().shaper_page().fill_revealer().set_reveal_child(true);
                },
                "ellipse" => {
                    appwindow.penssidebar().shaper_page().ellipse_toggle().set_active(true);
                    appwindow.canvas().pens().borrow_mut().shaper.current_shape = shaper::CurrentShape::Ellipse;
                    appwindow.penssidebar().shaper_page().fill_revealer().set_reveal_child(true);
                },
                _ => { log::error!("set invalid state of action `current-shape`")}
            }
        }),
    );
    app.add_action(&action_current_shape);

    // Delete Selection
    action_delete_selection.connect_activate(
        clone!(@weak appwindow => move |_action_delete_selection, _| {
                    let mut strokes = appwindow.canvas().sheet().selection().remove_strokes();
                    appwindow.canvas().sheet().strokes_trash().borrow_mut().append(&mut strokes);
        }),
    );
    app.add_action(&action_delete_selection);

    // Duplicate Selection
    action_duplicate_selection.connect_activate(
        clone!(@weak appwindow => move |_action_duplicate_selection, _| {
                    let mut strokes = (*appwindow.canvas().sheet().selection().strokes().borrow()).clone();
                    appwindow.canvas().sheet().strokes().borrow_mut().append(&mut strokes);

                    let offset = na::vector![20.0, 20.0];
                    appwindow.canvas().sheet().selection().translate_selection(offset);
        }),
    );
    app.add_action(&action_duplicate_selection);

    // Format borders
    action_sheet_format_borders.connect_state_notify(
        clone!(@weak appwindow => move |action_sheet_format_borders| {
            let state = action_sheet_format_borders.state().unwrap().get::<bool>().unwrap();
                appwindow.canvas().set_format_borders(state);
                appwindow.canvas().queue_draw();
        }),
    );
    app.add_action(&action_sheet_format_borders);

    // About Dialog
    action_about.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_about(&appwindow);
    }));
    app.add_action(&action_about);

    // Autoexpand height
    action_autoexpand_height.connect_state_notify(
        clone!(@weak appwindow => move |action_autoexpand_height| {
            let state = action_autoexpand_height.state().unwrap().get::<bool>().unwrap();

            appwindow.canvas().sheet().set_autoexpand_height(state);
            appwindow.mainheader().pageedit_revealer().set_reveal_child(!state);

            appwindow.canvas().queue_resize();
            appwindow.canvas().queue_draw();
        }),
    );
    app.add_action(&action_autoexpand_height);

    // Righthanded
    action_righthanded.connect_state_notify(clone!(@weak appwindow => move |action_righthanded| {

        if action_righthanded.state().unwrap().get::<bool>().unwrap() {
            appwindow.mainheader().canvasmenu().righthanded_toggle().set_active(true);

            appwindow.main_grid().remove::<Grid>(&appwindow.sidebar_grid());
            appwindow.main_grid().remove::<Separator>(&appwindow.sidebar_sep());
            appwindow.main_grid().remove::<Revealer>(&appwindow.devel_actions_revealer());
            appwindow.main_grid().remove::<ScrolledWindow>(&appwindow.canvas_scroller());
            appwindow.main_grid().attach(&appwindow.sidebar_grid(), 0, 1 ,1, 2);
            appwindow.main_grid().attach(&appwindow.sidebar_sep(), 1, 1 ,1, 2);
            appwindow.main_grid().attach(&appwindow.devel_actions_revealer(), 2, 1 ,1, 1);
            appwindow.main_grid().attach(&appwindow.canvas_scroller(), 2, 2 ,1, 1);

            appwindow.mainheader().headerbar().remove::<gtk4::Box>(&appwindow.mainheader().pens_togglebox());
            appwindow.mainheader().headerbar().remove::<gtk4::Box>(&appwindow.mainheader().quickactions_box());
            appwindow.mainheader().headerbar().pack_end::<gtk4::Box>(&appwindow.mainheader().quickactions_box());
            appwindow.mainheader().headerbar().pack_start::<gtk4::Box>(&appwindow.mainheader().pens_togglebox());

            appwindow.penssidebar().marker_page().colorpicker().set_property("position", PositionType::Left.to_value()).unwrap();
            appwindow.penssidebar().brush_page().templatechooser().help_button().set_direction(ArrowType::Right);
            appwindow.penssidebar().brush_page().templatechooser().chooser_button().set_direction(ArrowType::Right);
            appwindow.penssidebar().brush_page().colorpicker().set_property("position", PositionType::Left.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().stroke_colorpicker().set_property("position", PositionType::Left.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().fill_colorpicker().set_property("position", PositionType::Left.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().roughconfig_menubutton().set_direction(ArrowType::Right);

            appwindow.flap().set_flap_position(PackType::Start);
        } else {
            appwindow.mainheader().canvasmenu().lefthanded_toggle().set_active(true);

            appwindow.main_grid().remove::<Revealer>(&appwindow.devel_actions_revealer());
            appwindow.main_grid().remove::<ScrolledWindow>(&appwindow.canvas_scroller());
            appwindow.main_grid().remove::<Separator>(&appwindow.sidebar_sep());
            appwindow.main_grid().remove::<Grid>(&appwindow.sidebar_grid());
            appwindow.main_grid().attach(&appwindow.devel_actions_revealer(), 0, 1 ,1, 1);
            appwindow.main_grid().attach(&appwindow.canvas_scroller(), 0, 2 ,1, 1);
            appwindow.main_grid().attach(&appwindow.sidebar_sep(), 1, 1 ,1, 2);
            appwindow.main_grid().attach(&appwindow.sidebar_grid(), 2, 1 ,1, 2);

            appwindow.mainheader().headerbar().remove::<gtk4::Box>(&appwindow.mainheader().pens_togglebox());
            appwindow.mainheader().headerbar().remove::<gtk4::Box>(&appwindow.mainheader().quickactions_box());
            appwindow.mainheader().headerbar().pack_start::<gtk4::Box>(&appwindow.mainheader().quickactions_box());
            appwindow.mainheader().headerbar().pack_end::<gtk4::Box>(&appwindow.mainheader().pens_togglebox());

            appwindow.penssidebar().marker_page().colorpicker().set_property("position", PositionType::Right.to_value()).unwrap();
            appwindow.penssidebar().brush_page().templatechooser().help_button().set_direction(ArrowType::Left);
            appwindow.penssidebar().brush_page().templatechooser().chooser_button().set_direction(ArrowType::Left);
            appwindow.penssidebar().brush_page().colorpicker().set_property("position", PositionType::Right.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().stroke_colorpicker().set_property("position", PositionType::Right.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().fill_colorpicker().set_property("position", PositionType::Right.to_value()).unwrap();
            appwindow.penssidebar().shaper_page().roughconfig_menubutton().set_direction(ArrowType::Left);

            appwindow.flap().set_flap_position(PackType::End);
        }
    }));
    app.add_action(&action_righthanded);

    // Clear sheet
    action_clear_sheet.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_clear_sheet(&appwindow);
    }));
    app.add_action(&action_clear_sheet);

    // Open Canvas Menu
    action_open_canvasmenu.connect_activate(clone!(@weak appwindow => move |_,_| {
        appwindow.mainheader().canvasmenu().popovermenu().popup();
    }));
    app.add_action(&action_open_canvasmenu);

    // Open App Menu
    action_open_appmenu.connect_activate(clone!(@weak appwindow => move |_,_| {
        appwindow.mainheader().appmenu().popovermenu().popup();
    }));
    app.add_action(&action_open_appmenu);

    // Zoom reset
    action_zoom_reset.connect_activate(clone!(@weak appwindow => move |_,_| {
        appwindow.canvas().scale_to(Canvas::SCALE_DEFAULT);
        appwindow.canvas().regenerate_background(true, true);
    }));
    app.add_action(&action_zoom_reset);

    // Zoom fit to width
    action_zoom_fit_width.connect_activate(clone!(@weak appwindow => move |_,_| {
        let new_scalefactor = (appwindow.canvas_scroller().width() as f64 - Canvas::SHADOW_WIDTH * 2.0) / appwindow.canvas().sheet().format().width() as f64;
        appwindow.canvas().scale_to(new_scalefactor);
        appwindow.canvas().regenerate_background(true, true);
    }));
    app.add_action(&action_zoom_fit_width);

    // Zoom in
    action_zoomin.connect_activate(clone!(@weak appwindow => move |_,_| {
        let new_scalefactor = appwindow.canvas().scalefactor() * appwindow.canvas().temporary_zoom() + Canvas::ZOOM_ACTION_DELTA;
        appwindow.canvas().scale_to(new_scalefactor);
        appwindow.canvas().regenerate_background(true, true);
    }));
    app.add_action(&action_zoomin);

    // Zoom out
    action_zoomout.connect_activate(clone!(@weak appwindow => move |_,_| {
        let new_scalefactor = appwindow.canvas().scalefactor() * appwindow.canvas().temporary_zoom() - Canvas::ZOOM_ACTION_DELTA;
        appwindow.canvas().scale_to(new_scalefactor);
        appwindow.canvas().regenerate_background(true, true);
    }));
    app.add_action(&action_zoomout);

    // Temporary Eraser
    let pen_tmp = Rc::new(Cell::new(PenStyle::default()));

    action_tmperaser.connect_activate(move |action_tmperaser, parameter| {
        if let Some(parameter) = parameter {
            if let Some(state) = parameter.get::<bool>() {
                action_tmperaser.change_state(&state.to_variant());
            }
        } else {
            let state = action_tmperaser.state().unwrap().get::<bool>().unwrap();
            action_tmperaser.change_state(&(!state).to_variant());
        }
    });
    action_tmperaser.connect_change_state(
        clone!(@strong pen_tmp, @weak appwindow => move |action_tmperaser, value| {
            if value.unwrap().get::<bool>().unwrap() != action_tmperaser.state().unwrap().get::<bool>().unwrap() {
                action_tmperaser.set_state(value.unwrap());

                if action_tmperaser.state().unwrap().get::<bool>().unwrap() {
                    pen_tmp.set(appwindow.canvas().current_pen().get());
                    appwindow.canvas().current_pen().set(PenStyle::Eraser);
                } else {
                    appwindow.canvas().current_pen().set(pen_tmp.get());
                };
            }
        }),
    );
    app.add_action(&action_tmperaser);

    // New sheet
    action_new_sheet.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_new_sheet(&appwindow);
    }));
    app.add_action(&action_new_sheet);

    // Open workspace
    action_open_workspace.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_open_workspace(&appwindow);
    }));
    app.add_action(&action_open_workspace);

    // Open sheet
    action_open_sheet.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_open_sheet(&appwindow);
    }));
    app.add_action(&action_open_sheet);

    // Save sheet
    action_save_sheet.connect_activate(clone!(@weak appwindow => move |_, _| {
        if appwindow.application().unwrap().downcast::<RnoteApp>().unwrap().output_file().borrow().is_none() {
            dialogs::dialog_save_sheet_as(&appwindow);
        }

        if let Some(output_file) = appwindow.application().unwrap().downcast::<RnoteApp>().unwrap().output_file().borrow().to_owned() {
            if let Err(e) = appwindow.canvas().sheet().save_sheet(&output_file) {
                log::error!("failed to save sheet, {}", e);
                *appwindow.application().unwrap().downcast::<RnoteApp>().unwrap().output_file().borrow_mut() = None;
            } else {
            }
        }
    }));
    app.add_action(&action_save_sheet);

    // Save sheet as
    action_save_sheet_as.connect_activate(clone!(@weak appwindow => move |_, _| {
        dialogs::dialog_save_sheet_as(&appwindow);
    }));
    app.add_action(&action_save_sheet_as);

    // Print sheet
    action_print_sheet.connect_activate(clone!(@weak appwindow => move |_, _| {
        let print_op = PrintOperation::builder()
            .unit(Unit::Points)
            .n_pages(appwindow.canvas().sheet().calc_n_pages())
            .build();

/*         print_op.connect_begin_print(clone!(@weak appwindow => move |print_op, print_cx| {
            print_op.set_n_pages(appwindow.canvas().sheet().calc_n_pages());
        })); */

        print_op.connect_draw_page(clone!(@weak appwindow => move |_print_op, print_cx, page_nr| {
            log::debug!("draw-page signal callback started");

            let cx = match print_cx.cairo_context() {
                None => { return; }
                Some(cx) => { cx }
            };

            let (margin_top, margin_bottom, margin_left, margin_right) = print_cx.hard_margins().unwrap_or( (0.0, 0.0, 0.0, 0.0) );

            let width_scale = (print_cx.width() + margin_left + margin_right) / f64::from(appwindow.canvas().sheet().format().width());
            let height_scale = (print_cx.height() + margin_top + margin_bottom) / f64::from(appwindow.canvas().sheet().format().height());
            let print_scalefactor = width_scale.min(height_scale);
            let y_offset = - (f64::from(page_nr * appwindow.canvas().sheet().format().height()) * print_scalefactor);

            let app_scalefactor = appwindow.canvas().scalefactor();
            appwindow.canvas().scale_to(print_scalefactor);
            appwindow.canvas().regenerate_content(true, false);

            let snapshot = Snapshot::new();

            let format_bounds_scaled = p2d::bounding_volume::AABB::new(
                na::point![0.0, 0.0],
                na::point![f64::from(appwindow.canvas().sheet().format().width()) * print_scalefactor,f64::from(appwindow.canvas().sheet().format().height()) * print_scalefactor]
            );
            let sheet_size_scaled = na::vector![
                f64::from(appwindow.canvas().sheet().width()) * print_scalefactor,
                f64::from(appwindow.canvas().sheet().height()) * print_scalefactor
            ];

            appwindow.canvas().preview().snapshot(
                snapshot.dynamic_cast_ref::<gdk::Snapshot>().unwrap(),
                sheet_size_scaled[0],
                sheet_size_scaled[1],
            );

            cx.rectangle(
                format_bounds_scaled.mins[0],
                format_bounds_scaled.mins[1],
                format_bounds_scaled.maxs[0] - format_bounds_scaled.mins[0],
                format_bounds_scaled.maxs[1] - format_bounds_scaled.mins[1]
            );
            cx.clip();
            cx.translate(0.0, y_offset);

            if let Some(node) = snapshot.free_to_node() {
                node.draw(&cx);
            } else {
                log::error!("failed to get rendernode for created snapshot while printing page no {}", page_nr);
            };

            appwindow.canvas().scale_to(app_scalefactor);
            appwindow.canvas().regenerate_content(true, true);
        }));

        if let Err(e) = print_op.run(PrintOperationAction::PrintDialog, Some(&appwindow)){
            log::error!("failed to print, {}", e);
        };

    }));
    app.add_action(&action_print_sheet);

    // Import
    action_import_file.connect_activate(clone!(@weak appwindow => move |_,_| {
        dialogs::dialog_import_file(&appwindow);
    }));
    app.add_action(&action_import_file);

    // Export selection as SVG
    action_export_selection_as_svg.connect_activate(clone!(@weak appwindow => move |_,_| {
        dialogs::dialog_export_selection(&appwindow);
    }));
    app.add_action(&action_export_selection_as_svg);

    // Export sheet as SVG
    action_export_sheet_as_svg.connect_activate(clone!(@weak appwindow => move |_,_| {
        dialogs::dialog_export_sheet(&appwindow);
    }));
    app.add_action(&action_export_sheet_as_svg);
}

// ### Accelerators / Keyboard Shortcuts
pub fn setup_accels(appwindow: &RnoteAppWindow) {
    let app = appwindow
        .application()
        .unwrap()
        .downcast::<RnoteApp>()
        .unwrap();

    app.set_accels_for_action("app.keyboard-shortcuts", &["<Ctrl>question"]);
    app.set_accels_for_action("app.quit", &["<Ctrl>q"]);
    app.set_accels_for_action("app.open-canvasmenu", &["F9"]);
    app.set_accels_for_action("app.open-appmenu", &["F10"]);
    app.set_accels_for_action("app.new-sheet", &["<Ctrl>n"]);
    app.set_accels_for_action("app.open-sheet", &["<Ctrl>o"]);
    app.set_accels_for_action("app.save-sheet", &["<Ctrl>s"]);
    app.set_accels_for_action("app.save-sheet-as", &["<Ctrl><Shift>s"]);
    app.set_accels_for_action("app.clear-sheet", &["<Ctrl>l"]);
    app.set_accels_for_action("app.print-sheet", &["<Ctrl>p"]);
    app.set_accels_for_action("app.import-file", &["<Ctrl>i"]);
    app.set_accels_for_action("app.zoomin", &["plus"]);
    app.set_accels_for_action("app.zoomout", &["minus"]);
    app.set_accels_for_action("app.delete-selection", &["Delete"]);
    app.set_accels_for_action("app.duplicate-selection", &["<Ctrl>v"]);
    app.set_accels_for_action("app.tmperaser(true)", &["d"]);
}
