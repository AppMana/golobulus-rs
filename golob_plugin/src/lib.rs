pub(crate) mod background_task;
pub(crate) mod error;
pub(crate) mod footage_utils;
pub(crate) mod idle_task;
pub(crate) mod instance;
pub(crate) mod param_util;
mod setup_env;
pub(crate) mod ui;

use after_effects as ae;
use after_effects_sys as ae_sys;
use background_task::{BackgroundTask, JobId};
use idle_task::IdleTaskBundle;
use instance::{DebugContents, Instance, InstanceId};
use std::cell::Cell;
use std::sync::Arc;

#[repr(i32)]
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Eq)]
pub enum ParamIdx {
    ScriptGroupStart = 1,
    LoadButton,
    UnloadButton,
    SetVenv,
    UnsetVenv,
    ReloadButton,
    ScriptGroupEnd,
    IsImageFilter,
    DebugGroupBegin,
    ShowDebug,
    DebugOffset,
    TemporalWindow,
    DebugGroupEnd,
    ContinuousRenderGroupBegin,
    StartRender,
    CancelRender,
    ContinuousRenderGroupEnd,
    ParametersStart,
    ParametersEnd,
    Dynamic(i32),
}

#[derive(Default)]
struct GlobalPlugin {
    pub task_map: Arc<dashmap::DashMap<JobId, BackgroundTask>>,
    pub errors: Arc<dashmap::DashMap<InstanceId, HashMap<i32, DebugContents>>>,
    pub current_id: usize,
}

impl GlobalPlugin {
    pub fn bg_render_is_active(&self, id: JobId) -> bool {
        self.task_map.contains_key(&id)
    }

    pub fn render_progress(&self, id: JobId) -> Option<f32> {
        MAIN_THREAD_IDLE_DATA.with(|data| {
            let data = data.borrow();
            if let Some(task) = data.get(&id) {
                let ctx = &task.task_creation_ctx;
                Some(100.0 * ctx.current_frame as f32 / ctx.total_frames as f32)
            } else {
                None
            }
        })
    }
}

const INPUT_LAYER_CHECKOUT_ID: ParamIdx = ParamIdx::Dynamic(255);

static PLUGIN_ID: std::sync::OnceLock<i32> = std::sync::OnceLock::new();

thread_local! {
    // This is only ever set on the main / UI thread
    static IDLE_TASK_PICA: Cell<Option<*const ae_sys::SPBasicSuite>> = const { Cell::new(None) };
    static MAIN_THREAD_IDLE_DATA: RefCell<HashMap<JobId, IdleTaskBundle>> =  RefCell::new(HashMap::new()) ;
}

ae::define_effect!(GlobalPlugin, Instance, ParamIdx);

impl AdobePluginGlobal for GlobalPlugin {
    fn can_load(_host_name: &str, _host_version: &str) -> bool {
        true
    }

    fn params_setup(
        &self,
        params: &mut ae::Parameters<ParamIdx>,
        in_data: ae::InData,
        _: ae::OutData,
    ) -> Result<(), ae::Error> {
        param_util::setup_static_params(params)?;
        param_util::create_variant_backing(params)?;

        in_data.interact().register_ui(
            CustomUIInfo::new().events(ae::CustomEventFlags::COMP | ae::CustomEventFlags::EFFECT),
        )?;

        Ok(())
    }

    fn handle_command(
        &mut self,
        cmd: ae::Command,
        in_data: ae::InData,
        mut out_data: ae::OutData,
        _params: &mut ae::Parameters<ParamIdx>,
    ) -> Result<(), ae::Error> {
        match cmd {
            Command::About => {
                out_data.set_return_msg("Golobulus: The adder plods where it ought not.");
            }
            Command::GlobalSetup => {
                #[cfg(target_os = "macos")]
                env_logger::init();

                setup_env::set_up_env()?;

                let suite = ae::aegp::suites::Utility::new()?;

                PLUGIN_ID
                    .set(suite.register_with_aegp(None, "Golobulus")?)
                    .unwrap();

                IDLE_TASK_PICA.set(Some(in_data.pica_basic_suite_ptr()));
                // run a task every 1800 ms on the main thread
                idle_task::register(idle_task::IdleTaskInfo {
                    task_map: self.task_map.clone(),
                })?;
            }
            _ => {}
        };
        Ok(())
    }
}

impl AdobePluginInstance for Instance {
    fn handle_command(&mut self, plugin: &mut PluginState, command: Command) -> Result<(), Error> {
        match command {
            Command::About => plugin
                .out_data
                .set_return_msg("Golobulus: The adder plods where it ought not."),
            Command::Event { mut extra } => {
                ui::draw(
                    &plugin.in_data,
                    self,
                    plugin.global,
                    plugin.params,
                    &mut extra,
                )?;
            }
            Command::UpdateParamsUi => {
                param_util::update_param_defaults_and_labels(plugin, self)?;
                param_util::update_input_visibilities(plugin, self)?;
            }
            Command::UserChangedParam { param_index } => {
                let idx = ParamIdx::from(param_index);
                self.handle_param_interaction(plugin, idx)?;
            }
            Command::SmartPreRender { mut extra } => {
                self.smart_pre_render(&plugin.in_data, &mut extra)?;
            }
            Command::SmartRender { extra } => {
                let cb = extra.callbacks();
                self.smart_render(&plugin.in_data, plugin.global, &cb)?;
            }
            Command::SequenceSetup => {
                self.id = fastrand::usize(..);
                if let Some(src) = self.src.clone() {
                    self.runner.load_script(src, None).map_err(|e| {
                        error::startup_error_message(e, &mut plugin.out_data);
                        Error::Generic
                    })?;
                }
            }
            Command::SequenceResetup => {
                if let Some(venv_path) = self.venv_path.as_ref() {
                    let venv_path = std::path::PathBuf::from(venv_path);
                    self.runner.set_venv_path(venv_path);
                }

                if let Some(last_known) = self.last_known_path.as_ref() {
                    let file_path = std::path::PathBuf::from(last_known);
                    if let Some(parent) = file_path.parent() {
                        self.runner.set_script_parent_directory(parent.to_owned());
                    }
                }

                if let Some(src) = self.src.clone() {
                    self.runner.load_script(src, None).map_err(|e| {
                        error::startup_error_message(e, &mut plugin.out_data);
                        Error::Generic
                    })?;
                }
            }
            _ => {}
        };

        Ok(())
    }

    fn flatten(&self) -> Result<(u16, Vec<u8>), Error> {
        let out = bincode::serialize(&self).map_err(|_| {
            log::error!("Serialization Error");
            Error::Generic
        })?;
        Ok((1, out))
    }

    fn unflatten(version: u16, serialized: &[u8]) -> Result<Self, Error> {
        match version {
            1 => {
                let out: Self = bincode::deserialize(serialized).map_err(|_| Error::Generic)?;
                Ok(out)
            }
            _ => {
                log::error!("Deserialization Error");
                Err(Error::Generic)
            }
        }
    }

    fn render(&self, _: &mut PluginState, _: &Layer, _: &mut Layer) -> Result<(), ae::Error> {
        Ok(())
    }

    fn do_dialog(&mut self, _: &mut PluginState) -> Result<(), ae::Error> {
        Ok(())
    }
}
