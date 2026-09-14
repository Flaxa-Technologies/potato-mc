use std::{
    any::Any,
    sync::{Arc, LazyLock},
};

use libloading::Library;

use crate::plugin::{
    PLUGIN_API_VERSION,
    loader::{PluginLoadFuture, PluginUnloadFuture},
};

use super::{LoaderError, Path, Plugin, PluginLoader, PluginMetadata};

pub struct NativePluginLoader;

impl PluginLoader for NativePluginLoader {
    fn load<'a>(&'a self, path: &'a Path) -> PluginLoadFuture<'a> {
        Box::pin(async {
            let path = path.to_owned();

            // SAFETY: Loading dynamic library from path configured by server administrator.
            let library = unsafe { Library::new(&path) }
                .map_err(|e| LoaderError::LibraryLoad(e.to_string()))?;

            // 1. Check if this is a Potato native plugin
            if let Ok(potato_version_sym) = unsafe { library.get::<*const u32>(b"POTATO_API_VERSION") } {
                let potato_version = unsafe { **potato_version_sym };
                if potato_version != potato_api::POTATO_API_VERSION {
                    return Err(LoaderError::ApiVersionMismatch {
                        plugin_version: potato_version,
                        server_version: potato_api::POTATO_API_VERSION,
                    });
                }

                let plugin_factory = unsafe {
                    library
                        .get::<fn() -> Box<dyn potato_api::Plugin>>(b"potato_create_plugin")
                        .map_err(|_| LoaderError::EntrypointMissing)?
                };

                let potato_plugin_instance = plugin_factory();
                let potato_meta = potato_plugin_instance.metadata();

                let pumpkin_metadata = PluginMetadata {
                    name: potato_meta.name,
                    version: potato_meta.version,
                    authors: potato_meta.authors,
                    description: potato_meta.description,
                    dependencies: potato_meta.dependencies,
                    permissions: Vec::new(),
                };

                let adapter = crate::plugin::potato_host::PotatoPluginAdapter::new(potato_plugin_instance);

                return Ok((
                    Arc::new(adapter) as Arc<dyn Plugin>,
                    pumpkin_metadata,
                    Box::new(library) as Box<dyn Any + Send + Sync>,
                ));
            }

            // Fallback: Check for legacy Pumpkin plugin
            let plugin_api_version = unsafe {
                match library.get::<*const u32>(b"PUMPKIN_API_VERSION") {
                    Ok(symbol) => **symbol,
                    Err(_) => return Err(LoaderError::ApiVersionMissing),
                }
            };

            if plugin_api_version != PLUGIN_API_VERSION {
                return Err(LoaderError::ApiVersionMismatch {
                    plugin_version: plugin_api_version,
                    server_version: PLUGIN_API_VERSION,
                });
            }

            // 2. Extract Metadata (METADATA)
            // `#[plugin_impl]` exports this as a `LazyLock`, since `PluginMetadata`
            // owns its strings and can't be built in a const.
            // SAFETY: `METADATA` is an exported `LazyLock<PluginMetadata>` symbol created by `#[plugin_impl]`.
            let metadata = unsafe {
                let metadata = library
                    .get::<*const LazyLock<PluginMetadata>>(b"METADATA")
                    .map_err(|_| LoaderError::MetadataMissing)?;
                (**metadata).clone()
            };

            // 3. Extract Plugin Factory (plugin)
            // SAFETY: `plugin` is an exported constructor function symbol with signature `fn() -> Box<dyn Plugin>` created by `#[plugin_impl]`.
            let plugin_factory = unsafe {
                library
                    .get::<fn() -> Box<dyn Plugin>>(b"plugin")
                    .map_err(|_| LoaderError::EntrypointMissing)?
            };

            Ok((
                Arc::from(plugin_factory()),
                metadata,
                Box::new(library) as Box<dyn Any + Send + Sync>,
            ))
        })
    }

    fn can_load(&self, path: &Path) -> bool {
        let ext = path.extension().unwrap_or_default();

        if cfg!(target_os = "windows") {
            ext.eq_ignore_ascii_case("dll")
        } else if cfg!(target_os = "macos") {
            ext.eq_ignore_ascii_case("dylib")
        } else {
            ext.eq_ignore_ascii_case("so")
        }
    }

    fn unload(&self, data: Box<dyn Any + Send + Sync>) -> PluginUnloadFuture<'_> {
        Box::pin(async {
            data.downcast::<Library>()
                .map_or(Err(LoaderError::InvalidLoaderData), |library| {
                    drop(library);
                    Ok(())
                })
        })
    }

    /// Windows specific issue: Windows locks DLLs, so we must indicate they cannot be unloaded.
    fn can_unload(&self) -> bool {
        !cfg!(target_os = "windows")
    }
}
