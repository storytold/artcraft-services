// Compiled by sync-launcher-catalog.py against a pinned launcher checkout.
use artcraft_launcher_core::{catalog, details, desktop_entry};

fn main() {
    for app in &catalog::SUITE {
        println!("\n[[apps]]");
        macro_rules! field {
            ($key:ident, $value:expr) => { println!("{} = {:?}", stringify!($key), $value); };
        }
        if let Some(bundle) = app.bundle { field!(bundle, bundle); }
        field!(color, app.color);
        field!(former_bundles, app.former_bundles);
        field!(former_ids, app.former_ids);
        field!(former_names, app.former_names);
        field!(id, app.id);
        field!(binary, app.id);
        field!(flatpak_id, app.bundle_id());
        field!(mac_bundle_id, app.bundle_id());
        field!(mac_bundle_name, app.mac_bundle_name());
        field!(website, app.website());
        field!(windows_msi_publisher, app.msi_publisher.unwrap_or("Learning Machines LLC"));
        field!(deb_package, app.id);
        field!(rpm_package, app.id);
        let (categories, wm_class) = desktop_entry::meta(app);
        field!(desktop_categories, categories.split(';').filter(|c| !c.is_empty()).collect::<Vec<_>>());
        field!(startup_wm_class, wm_class.map(str::to_string).unwrap_or_else(|| app.bundle_id()));
        field!(ink_dark, app.ink_dark);
        field!(ink_light, app.ink_light);
        field!(kind, match app.kind { catalog::AppKind::Official => "official", catalog::AppKind::Community => "community" });
        if let Some(team) = app.mac_team { field!(mac_team, team); }
        if let Some(publisher) = app.msi_publisher { field!(msi_publisher, publisher); }
        field!(name, app.name);
        field!(publisher, app.publisher);
        field!(repo, app.repo);
        if let Some(site) = app.site { field!(site, site); }
        field!(tagline, app.tagline);
        if let Some(page) = details::AppDetails::by_id(app.id) {
            println!("[apps.details]");
            field!(headline, page.headline);
            field!(about, page.about);
            field!(platforms, page.platforms);
            for feature in page.features {
                println!("[[apps.details.features]]");
                field!(text, feature.text);
                field!(title, feature.title);
            }
            for shot in page.shots {
                println!("[[apps.details.shots]]");
                field!(caption, shot.caption);
                field!(key, shot.key);
                if let Some(url) = page.screenshot_url(shot.key) { field!(url, url); }
            }
        }
    }
}
