use crate::*;

use gtk::{self, prelude::*};

/// Recognised values of org.freedesktop.StatusNotifierItem.Status
///
/// See
/// <https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/#org.freedesktop.statusnotifieritem.status>
/// for details.
#[derive(Debug, Clone, Copy)]
pub enum Status {
    /// The item doesn't convey important information to the user, it can be considered an "idle"
    /// status and is likely that visualizations will chose to hide it.
    Passive,
    /// The item is active, is more important that the item will be shown in some way to the user.
    Active,
    /// The item carries really important information for the user, such as battery charge running
    /// out and is wants to incentive the direct user intervention. Visualizations should emphasize
    /// in some way the items with NeedsAttention status.
    NeedsAttention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseStatusError;

impl std::str::FromStr for Status {
    type Err = ParseStatusError;

    fn from_str(s: &str) -> std::result::Result<Self, ParseStatusError> {
        match s {
            "Passive" => Ok(Status::Passive),
            "Active" => Ok(Status::Active),
            "NeedsAttention" => Ok(Status::NeedsAttention),
            _ => Err(ParseStatusError),
        }
    }
}

/// Split a sevice name e.g. `:1.50:/org/ayatana/NotificationItem/nm_applet` into the address and
/// path.
///
/// Original logic from <https://github.com/oknozor/stray/blob/main/stray/src/notifier_watcher/notifier_address.rs>
fn split_service_name(service: &str) -> zbus::Result<(String, String)> {
    if let Some((addr, path)) = service.split_once('/') {
        Ok((addr.to_owned(), format!("/{}", path)))
    } else if service.contains(':') {
        // TODO why?
        let addr = service.split(':').nth(1);
        // Some StatusNotifierItems will not return an object path, in that case we fallback
        // to the default path.
        if let Some(addr) = addr {
            Ok((addr.to_owned(), "/StatusNotifierItem".to_owned()))
        } else {
            Err(zbus::Error::Address(service.to_owned()))
        }
    } else {
        Err(zbus::Error::Address(service.to_owned()))
    }
}

pub struct Item {
    pub sni: dbus::StatusNotifierItemProxy<'static>,
    gtk_menu: Option<dbusmenu_gtk3::Menu>,
}

impl Item {
    pub async fn from_address(con: &zbus::Connection, addr: &str) -> zbus::Result<Self> {
        let (addr, path) = split_service_name(addr)?;
        let sni = dbus::StatusNotifierItemProxy::builder(con).destination(addr)?.path(path)?.build().await?;

        Ok(Self { sni, gtk_menu: None })
    }

    /// Get the current status of the item.
    pub async fn status(&self) -> zbus::Result<Status> {
        let status = self.sni.status().await?;
        match status.parse() {
            Ok(s) => Ok(s),
            Err(_) => Err(zbus::Error::Failure(format!("Invalid status {:?}", status))),
        }
    }

    pub async fn set_menu(&mut self) -> zbus::Result<()> {
        let menu = self.sni.menu().await?;
        self.gtk_menu = Some(dbusmenu_gtk3::Menu::new(self.sni.destination(), &menu));
        Ok(())
    }

    pub async fn popup_menu(&self, widget: &gtk::EventBox, event: &gdk::EventButton, x: i32, y: i32) -> zbus::Result<()> {
        if let Some(menu) = &self.gtk_menu {
            menu.set_attach_widget(Some(widget));
            menu.popup_at_pointer(event.downcast_ref::<gdk::Event>());
            Ok(())
        } else {
            self.sni.context_menu(x, y).await
        }
    }

    pub async fn icon(&self, size: i32, scale: i32) -> Option<gtk::gdk_pixbuf::Pixbuf> {
        // see icon.rs
        load_icon_from_sni(&self.sni, size, scale).await
    }
}
