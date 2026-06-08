                    (KeyCode::Char('y'), _) => {
                        app.selected_panel = app::Panel::Timeline;
                        app.timeline_view = app::TimelineView::Yesterday;
                        app.timeline_scroll = 0;
                    }
