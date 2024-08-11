// SPDX-License-Identifier: GPL-3.0-only

use std::time;
use sysinfo::System;

use cosmic::app::{Command, Core};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{subscription, Limits};
use cosmic::iced_style::application;
use cosmic::widget::{self, settings, svg};
use cosmic::{Element, Theme};

//use chrono::{Datelike, DurationRound, Timelike};
use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{column, row, vertical_space},
        Alignment, Length, Subscription,
    },
    iced_widget::Column,
    widget::{container, horizontal_space},
};

use crate::fl;
use crate::svgstat::SvgStat;

/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Lib for retrieving system stats
    system: System,
    /// Current Total Load Avg in %
    cpu_load: f32,
    /// Current Mem usage in GB
    mem_usage: f64,
    /// The svg image to draw
    svgstat: super::svgstat::SvgStat,
    /// The popup id.
    popup: Option<Id>,
    /// Text mode toggler.
    text_only: bool,
    /// CPU load toggler.
    enable_cpu: bool,
    /// Memory load toggler.
    enable_mem: bool,
}

/// This is the enum that contains all the possible variants that your application will need to transmit messages.
/// This is used to communicate between the different parts of your application.
/// If your application does not need to send messages, you can use an empty enum or `()`.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    Tick,
    PopupClosed(Id),
    ToggleTextMode(bool),
    ToggleCpu(bool),
    ToggleMemory(bool),

}

/// Implement the `Application` trait for your application.
/// This is where you define the behavior of your application.
///
/// The `Application` trait requires you to define the following types and constants:
/// - `Executor` is the async executor that will be used to run your application's commands.
/// - `Flags` is the data that your application needs to use before it starts.
/// - `Message` is the enum that contains all the possible variants that your application will need to transmit messages.
/// - `APP_ID` is the unique identifier of your application.
impl cosmic::Application for Minimon {
    type Executor = cosmic::SingleThreadExecutor;
    //    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.Minimon";

    /// This is the entry point of your application, it is where you initialize your application.
    ///
    /// Any work that needs to be done before the application starts should be done here.
    ///
    /// - `core` is used to passed on for you by libcosmic to use in the core of your own application.
    /// - `flags` is used to pass in any data that your application needs to use before it starts.
    /// - `Command` type is used to send messages to your application. `Command::none()` can be used to send no messages to your application.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let system = System::new();

        let app = Minimon {
            core,
            system,
            cpu_load: 0.0,
            mem_usage: 0.0,
            svgstat: super::svgstat::SvgStat::new(),
            popup: None,
            text_only: false,
            enable_cpu: true,
            enable_mem: true,

        };

        (app, Command::none())
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        println!("subscription");
        fn time_subscription() -> Subscription<()> {
            subscription::unfold("time-sub", (), move |()| async move {
                let duration = time::Duration::from_millis(1000);
                tokio::time::sleep(duration).await;
                ((), ())
            })
        }

        Subscription::batch(vec![time_subscription().map(|_| Message::Tick)])
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<Message> {
        let horizontal = matches!(
            self.core.applet.anchor,
            PanelAnchor::Top | PanelAnchor::Bottom
        );

        let button: cosmic::widget::Button<Message>;

/* 
    let s = match core::str::from_utf8(self.svgstat.as_bytes()) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };

        println!("SVG: {}", s);*/

        if !self.text_only {
            let handle = cosmic::widget::icon::from_svg_bytes(self.svgstat.as_bytes().to_owned());

            button = self
                .core
                .applet
                .icon_button_from_handle(handle)
                .on_press(Message::TogglePopup)
                .style(cosmic::theme::Button::AppletIcon);
        } else {
            button = cosmic::widget::button(if horizontal {
                let formated = format!("{:.2}% {:.1}GB", self.cpu_load, self.mem_usage);
                Element::from(
                    row!(
                        self.core.applet.text(formated),
                        container(vertical_space(Length::Fixed(
                            (self.core.applet.suggested_size(true).1
                                + 2 * self.core.applet.suggested_padding(true))
                                as f32
                        )))
                    )
                    .align_items(Alignment::Center),
                )
            } else {
                let formated_cpu: String;
                if self.cpu_load < 10 as f32 {
                    formated_cpu = format!("{:.2}%", self.cpu_load);
                } else {
                    formated_cpu = format!("{:.1}%", self.cpu_load);
                }
                let formated_mem = format!("{:.1}GB", self.mem_usage);

                // vertical layout
                let mut elements = Vec::new();

                elements.push(self.core.applet.text(formated_cpu.to_owned()).into());
                elements.push(self.core.applet.text(formated_mem.to_owned()).into());

                let col = Column::with_children(elements)
                    .align_items(Alignment::Center)
                    .spacing(0);

                Element::from(
                    column!(
                        col,
                        horizontal_space(Length::Fixed(
                            (self.core.applet.suggested_size(true).0
                                + 2 * self.core.applet.suggested_padding(true))
                                as f32
                        ))
                    )
                    .align_items(Alignment::Center),
                )
            })
            .padding(if horizontal {
                [0, self.core.applet.suggested_padding(true)]
            } else {
                [self.core.applet.suggested_padding(true), 0]
            })
            .on_press(Message::TogglePopup)
            .style(cosmic::theme::Button::AppletIcon);
        }

        //        if let Some(tracker) = self.rectangle_tracker.as_ref() {
        //          tracker.container(0, button).ignore_bounds(true).into()
        //    } else {
        button.into()
        //  }
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(settings::item(
                fl!("text-only"),
                widget::toggler(None, self.text_only, |value| {
                    Message::ToggleTextMode(value)
                }),
            ))
            .add(settings::item(
                fl!("enable-cpu"),
                widget::toggler(None, self.enable_cpu, |value| {
                    Message::ToggleCpu(value)
                }),
            ))
            .add(settings::item(
                fl!("enable-memory"),
                widget::toggler(None, self.enable_mem, |value| {
                    Message::ToggleMemory(value)
                }),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        println!("Tick");
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::Tick => {
                self.system.refresh_cpu_usage(); // Refreshing CPU usage.
                self.system.refresh_memory();

                self.cpu_load = self
                    .system
                    .cpus()
                    .iter()
                    .map(|p| p.cpu_usage())
                    .sum::<f32>()
                    / self.system.cpus().len() as f32;
                self.mem_usage = self.system.used_memory() as f64 / (1073741824) as f64;

                self.svgstat.set_variable(self.cpu_load);
                println!("Message::Tick {}% - {}GB ", self.cpu_load, self.mem_usage);
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleTextMode(toggled) => self.text_only = toggled,
            Message::ToggleCpu(toggled) => {
                self.enable_cpu = toggled;
                if toggled == false {
                    self.enable_mem = true;    
                }
            },
            Message::ToggleMemory(toggled) => {
                self.enable_mem = toggled;
                if toggled == false {
                    self.enable_cpu = true;    
                }
            },
        }
        Command::none()
    }
}
