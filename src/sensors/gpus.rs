use cosmic::Element;
use log::info;
use std::{collections::VecDeque, fmt::Write};

use cosmic::widget::{self, Column};
use cosmic::widget::{settings, toggler};

use cosmic::{
    iced::{
        Alignment,
        widget::{column, row},
    },
    iced_widget::Row,
};

use crate::app::Message;
use crate::colorpicker::DemoGraph;
use crate::config::DeviceKind;
use crate::{
    config::{ColorVariant, GraphColors, GraphKind},
    fl,
    svg_graph::SvgColors,
};

use super::gpu::amd::AmdGpu;
use super::gpu::intel::IntelGpu;
use super::gpu::{GpuIf, nvidia::NvidiaGpu};

use std::sync::LazyLock;

pub static COLOR_CHOICES_RING: LazyLock<[(&'static str, ColorVariant); 4]> = LazyLock::new(|| {
    [
        (fl!("graph-ring-r1").leak(), ColorVariant::Color4),
        (fl!("graph-ring-r2").leak(), ColorVariant::Color3),
        (fl!("graph-ring-back").leak(), ColorVariant::Color1),
        (fl!("graph-ring-text").leak(), ColorVariant::Color2),
    ]
});

pub static COLOR_CHOICES_LINE: LazyLock<[(&'static str, ColorVariant); 3]> = LazyLock::new(|| {
    [
        (fl!("graph-line-graph").leak(), ColorVariant::Color4),
        (fl!("graph-line-back").leak(), ColorVariant::Color1),
        (fl!("graph-line-frame").leak(), ColorVariant::Color2),
    ]
});

const GRAPH_OPTIONS: [&str; 2] = ["Ring", "Line"];

const MAX_SAMPLES: usize = 21;

pub struct GpuGraph {
    id: String,
    samples: VecDeque<f64>,
    graph_options: Vec<&'static str>,
    kind: GraphKind,
    colors: GraphColors,
    svg_colors: SvgColors,
    disabled: bool,
    disabled_colors: SvgColors,
}

impl GpuGraph {
    fn new(id: &str) -> Self {
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();
        GpuGraph {
            id: id.to_owned(),
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            graph_options: GRAPH_OPTIONS.to_vec(),
            kind: GraphKind::Ring,
            colors: GraphColors::default(),
            svg_colors: SvgColors::new(&GraphColors::default()),
            disabled: false,
            disabled_colors: SvgColors {
                color1: String::from("#FFFFFF20"),
                color2: String::from("#727272FF"),
                color3: String::from("#727272FF"),
                color4: String::from("#727272FF"),
            },
        }
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = 0.0;
        }
    }

    pub fn graph(&self) -> String {
        if self.kind == GraphKind::Ring {
            let latest = self.latest_sample();
            let mut value = String::with_capacity(10);
            let mut percentage = String::with_capacity(10);

            if self.disabled {
                _ = write!(percentage, "0");
                _ = write!(value, "-");
            } else {
                if latest < 10.0 {
                    write!(value, "{latest:.2}").unwrap();
                } else if latest < 100.0 {
                    write!(value, "{latest:.1}").unwrap();
                } else {
                    write!(value, "{latest}").unwrap();
                }
                write!(percentage, "{latest}").unwrap();
            }

            crate::svg_graph::ring(
                &value,
                &percentage,
                if self.disabled {
                    &self.disabled_colors
                } else {
                    &self.svg_colors
                },
            )
        } else {
            crate::svg_graph::line(
                &self.samples,
                100.0,
                if self.disabled {
                    &self.disabled_colors
                } else {
                    &self.svg_colors
                },
            )
        }
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn graph_kind(&self) -> crate::config::GraphKind {
        self.kind
    }

    pub fn set_graph_kind(&mut self, kind: crate::config::GraphKind) {
        self.kind = kind;
    }

    pub fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    pub fn string(&self) -> String {
        let current_val = self.latest_sample();
        let unit = "%";

        if current_val < 10.0 {
            format!("{current_val:.2}{unit}")
        } else if current_val < 100.0 {
            format!("{current_val:.1}{unit}")
        } else {
            format!("{current_val}{unit}")
        }
    }

    pub fn update(&mut self, sample: u32) {
        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(f64::from(sample));
    }
}

impl DemoGraph for GpuGraph {
    fn demo(&self) -> String {
        match self.kind {
            GraphKind::Ring => {
                // show a number of 40% of max
                let val = 40;
                let percentage = 40.0;
                crate::svg_graph::ring(
                    &format!("{val}"),
                    &format!("{percentage}"),
                    &self.svg_colors,
                )
            }
            GraphKind::Line => {
                crate::svg_graph::line(&VecDeque::from(DEMO_SAMPLES), 100.0, &self.svg_colors)
            }
            GraphKind::Heat => panic!("Wrong graph choice!"),
        }
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        if self.kind == GraphKind::Line {
            (*COLOR_CHOICES_LINE).into()
        } else {
            (*COLOR_CHOICES_RING).into()
        }
    }

    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

pub struct VramGraph {
    id: String,
    samples: VecDeque<f64>,
    graph_options: Vec<&'static str>,
    kind: GraphKind,
    max_val: f64,

    //colors
    colors: GraphColors,
    svg_colors: SvgColors,
    disabled: bool,
    disabled_colors: SvgColors,
}

impl VramGraph {
    // id: a unique id, total: RAM size in GB
    fn new(id: &str, total: f64) -> Self {
        VramGraph {
            id: id.to_owned(),
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            graph_options: GRAPH_OPTIONS.to_vec(),
            kind: GraphKind::Ring,
            max_val: total,
            colors: GraphColors::default(),
            svg_colors: SvgColors::new(&GraphColors::default()),
            disabled: false,
            disabled_colors: SvgColors {
                color1: String::from("#FFFFFF20"),
                color2: String::from("#727272FF"),
                color3: String::from("#727272FF"),
                color4: String::from("#727272FF"),
            },
        }
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = 0.0;
        }
    }

    pub fn graph(&self) -> String {
        if self.kind == GraphKind::Ring {
            let latest = self.latest_sample();
            let mut value = String::with_capacity(10);
            let mut percentage = String::with_capacity(10);

            if self.disabled {
                _ = write!(percentage, "0");
                _ = write!(value, "-");
            } else {
                let pct: u64 = ((latest / self.max_val) * 100.0) as u64;

                write!(percentage, "{pct}").unwrap();

                if latest < 10.0 {
                    write!(value, "{latest:.2}").unwrap();
                } else if latest < 100.0 {
                    write!(value, "{latest:.1}").unwrap();
                } else {
                    write!(value, "{latest}").unwrap();
                }
            }
            crate::svg_graph::ring(
                &value,
                &percentage,
                if self.disabled {
                    &self.disabled_colors
                } else {
                    &self.svg_colors
                },
            )
        } else {
            crate::svg_graph::line(
                &self.samples,
                self.max_val,
                if self.disabled {
                    &self.disabled_colors
                } else {
                    &self.svg_colors
                },
            )
        }
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn graph_kind(&self) -> crate::config::GraphKind {
        self.kind
    }

    pub fn set_graph_kind(&mut self, kind: crate::config::GraphKind) {
        self.kind = kind;
    }

    pub fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    pub fn string(&self, vertical_panel: bool) -> String {
        let current_val = self.latest_sample();
        let unit: &str = if vertical_panel { "GB" } else { " GB" };

        if current_val < 10.0 {
            format!("{current_val:.2}{unit}")
        } else if current_val < 100.0 {
            format!("{current_val:.1}{unit}")
        } else {
            format!("{current_val}{unit}")
        }
    }

    pub fn total(&self) -> f64 {
        self.max_val
    }

    pub fn update(&mut self, sample: u64) {
        let new_val: f64 = sample as f64 / 1_073_741_824.0;

        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(new_val);
    }
}

impl DemoGraph for VramGraph {
    fn demo(&self) -> String {
        match self.kind {
            GraphKind::Ring => {
                // show a number of 40% of max
                let val = 40;
                let percentage = 40.0;
                crate::svg_graph::ring(
                    &format!("{val}"),
                    &format!("{percentage}"),
                    &self.svg_colors,
                )
            }
            GraphKind::Line => {
                crate::svg_graph::line(&VecDeque::from(DEMO_SAMPLES), 32.0, &self.svg_colors)
            }
            GraphKind::Heat => panic!("Wrong graph choice!"),
        }
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        if self.kind == GraphKind::Line {
            (*COLOR_CHOICES_LINE).into()
        } else {
            (*COLOR_CHOICES_RING).into()
        }
    }

    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

pub struct Gpu {
    gpu_if: Box<dyn GpuIf>,
    pub gpu: GpuGraph,
    pub vram: VramGraph,
    is_laptop: bool,
}

impl Gpu {
    pub fn new(gpu_if: Box<dyn GpuIf>) -> Self {
        let total = gpu_if.vram_total();
        let id = gpu_if.id();

        Gpu {
            gpu_if,
            gpu: GpuGraph::new(&id),
            vram: VramGraph::new(&id, total as f64 / 1_073_741_824.0),
            is_laptop: false,
        }
    }

    pub fn name(&self) -> String {
        self.gpu_if.as_ref().name().clone()
    }

    pub fn id(&self) -> String {
        self.gpu_if.as_ref().id().clone()
    }

    pub fn set_laptop(&mut self) {
        self.is_laptop = true;
    }

    pub fn demo_gpu_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = GpuGraph::new(&self.id());
        dmo.set_colors(colors);
        dmo.set_graph_kind(self.gpu.kind);
        Box::new(dmo)
    }

    pub fn demo_vram_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = VramGraph::new(&self.id(), self.vram.max_val);
        dmo.set_colors(colors);
        dmo.set_graph_kind(self.vram.kind);
        Box::new(dmo)
    }

    pub fn update(&mut self) {
        if self.gpu_if.is_active() {
            if let Ok(sample) = self.gpu_if.usage() {
                self.gpu.update(sample);
            }
            if let Ok(sample) = self.gpu_if.vram_used() {
                self.vram.update(sample);
            }
        }
    }

    pub fn restart(&mut self) {
        info!("Restarting {}", self.name());
        self.gpu_if.restart();
        self.gpu.disabled = false;
        self.vram.disabled = false;
    }

    pub fn stop(&mut self) {
        info!("Stopping {}", self.name());
        self.gpu_if.stop();
        self.gpu.clear();
        self.vram.clear();
        self.gpu.disabled = true;
        self.vram.disabled = true;
    }

    pub fn is_active(&self) -> bool {
        self.gpu_if.is_active()
    }

    pub fn settings_ui(
        &self,
        config: &crate::config::GpuConfig,
    ) -> cosmic::Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let battery_disable = if self.is_laptop {
            Some(settings::item(
                fl!("settings-disable-on-battery"),
                widget::toggler(config.pause_on_battery).on_toggle(move |value| {
                    Message::ToggleDisableOnBattery(self.id().clone(), value)
                }),
            ))
        } else {
            None
        };

        // GPU load
        let mut gpu_elements = Vec::new();

        let usage = self.gpu.string();
        gpu_elements.push(Element::from(
            column!(
                widget::svg(widget::svg::Handle::from_memory(
                    self.gpu.graph().as_bytes().to_owned(),
                ))
                .width(90)
                .height(60),
                cosmic::widget::text::body(usage),
            )
            .padding(cosmic::theme::spacing().space_xs)
            .align_x(Alignment::Center),
        ));

        let gpu_kind = self.gpu.graph_kind();
        let selected: Option<usize> = Some(gpu_kind.into());
        let id = self.id();
        gpu_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-gpu-chart"),
                    toggler(config.gpu_chart).on_toggle(move |value| {
                        Message::GpuToggleChart(self.id(), DeviceKind::Gpu, value)
                    }),
                ),
                settings::item(
                    fl!("enable-gpu-label"),
                    toggler(config.gpu_label).on_toggle(move |value| {
                        Message::GpuToggleLabel(self.id(), DeviceKind::Gpu, value)
                    }),
                ),
                row!(
                    widget::dropdown(&self.gpu.graph_options, selected, move |m| {
                        Message::GpuSelectGraphType(id.clone(), DeviceKind::Gpu, m.into())
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors")).on_press(
                        Message::ColorPickerOpen(DeviceKind::Gpu, gpu_kind, Some(self.id())),
                    )
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        let gpu = column![
            widget::text::heading(fl!("gpu-title-usage")),
            Row::with_children(gpu_elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xs())
        ]
        .spacing(cosmic::theme::spacing().space_xs);

        // VRAM load
        let mut vram_elements = Vec::new();
        let vram = self.vram.string(false);
        vram_elements.push(Element::from(
            column!(
                widget::svg(widget::svg::Handle::from_memory(
                    self.vram.graph().as_bytes().to_owned(),
                ))
                .width(90)
                .height(60),
                cosmic::widget::text::body(vram),
            )
            .padding(cosmic::theme::spacing().space_xs)
            .align_x(Alignment::Center),
        ));

        let selected: Option<usize> = Some(self.vram.graph_kind().into());
        let mem_kind = self.vram.graph_kind();
        let id = self.id();
        vram_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-vram-chart"),
                    toggler(config.vram_chart).on_toggle(|value| {
                        Message::GpuToggleChart(self.id(), DeviceKind::Vram, value)
                    }),
                ),
                settings::item(
                    fl!("enable-vram-label"),
                    toggler(config.vram_label).on_toggle(|value| {
                        Message::GpuToggleLabel(self.id(), DeviceKind::Vram, value)
                    }),
                ),
                row!(
                    widget::dropdown(&self.vram.graph_options, selected, move |m| {
                        Message::GpuSelectGraphType(id.clone(), DeviceKind::Vram, m.into())
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors")).on_press(
                        Message::ColorPickerOpen(DeviceKind::Vram, mem_kind, Some(self.id())),
                    )
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        let vram = column![
            widget::text::heading(fl!("gpu-title-vram")),
            Row::with_children(vram_elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xs())
        ]
        .spacing(cosmic::theme::spacing().space_xs);

        let stacked = if config.vram_label && config.gpu_label {
            Some(settings::item(
                fl!("settings-gpu-stack-labels"),
                row!(
                    widget::toggler(config.stack_labels).on_toggle(move |value| {
                        Message::GpuToggleStackLabels(self.id().clone(), value)
                    })
                ),
            ))
        } else {
            None
        };

        Column::new()
            .push_maybe(battery_disable)
            .push(gpu)
            .push(vram)
            .push_maybe(stacked)
            .spacing(cosmic::theme::spacing().space_xs)
            .into()
    }
}

pub fn list_gpus() -> Vec<Gpu> {
    let mut v: Vec<Gpu> = Vec::new();

    v.extend(IntelGpu::get_gpus());
    v.extend(NvidiaGpu::get_gpus());
    v.extend(AmdGpu::get_gpus());
    v
}

const DEMO_SAMPLES: [f64; 21] = [
    0.0,
    12.689857482910156,
    12.642768859863281,
    12.615306854248047,
    12.658184051513672,
    12.65273666381836,
    12.626102447509766,
    12.624862670898438,
    12.613967895507813,
    12.619949340820313,
    19.061111450195313,
    21.691085815429688,
    21.810935974121094,
    21.28915786743164,
    22.041973114013672,
    21.764171600341797,
    21.89263916015625,
    15.258216857910156,
    14.770732879638672,
    14.496528625488281,
    13.892818450927734,
];
