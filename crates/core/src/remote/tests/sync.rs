//! Refreshing a manifest's remotes: which catalogs a refresh reaches for,
//! and what an unreachable one does to the call.

use std::fs;

use super::{REPO, fixture};
use crate::manifest;
use crate::remote::{cache_head, sync_declared_sources, sync_sources};

/// Every enabled remote in a manifest resolves; a never-cached one that
/// cannot be reached fails the whole call rather than half-resolving.
#[test]
fn sync_sources_reports_warnings_and_fails_on_the_unreachable() {
    let f = fixture();
    let mut manifest = manifest::seed(&[]);
    manifest.sources.insert(
        "cat".to_owned(),
        manifest::SourceDecl {
            repo: Some(REPO.to_owned()),
            path: None,
            rev: None,
            enabled: true,
        },
    );
    manifest.sources.remove(manifest::DEFAULT_SOURCE_NAME);
    assert!(sync_sources(&f.env, &manifest).unwrap().is_empty());
    assert_eq!(cache_head(&f.env, REPO, None).unwrap().len(), 7);

    fs::remove_dir_all(&f.upstream).unwrap();
    assert_eq!(sync_sources(&f.env, &manifest).unwrap().len(), 1);

    manifest.sources.get_mut("cat").unwrap().repo = Some("owner/gone".to_owned());
    assert!(sync_sources(&f.env, &manifest).is_err());
}

/// A refresh fetches what this scope installs from, not every catalog the
/// manifest happens to name. A seeded manifest always carries the default
/// catalog, so fetching all of them lets a repository nobody installed from
/// fail — or merely slow — every refresh.
#[test]
fn a_refresh_skips_a_catalog_nothing_installs_from() {
    let f = fixture();
    let mut manifest = manifest::seed(&[]);
    manifest.sources.insert(
        "cat".to_owned(),
        manifest::SourceDecl {
            repo: Some(REPO.to_owned()),
            path: None,
            rev: None,
            enabled: true,
        },
    );
    // The seeded default is unreachable and nothing declares anything from it.
    manifest
        .sources
        .get_mut(manifest::DEFAULT_SOURCE_NAME)
        .unwrap()
        .repo = Some("owner/gone".to_owned());
    manifest
        .skills
        .insert("gh".to_owned(), manifest::ItemDecl::from_source("cat"));

    assert!(
        sync_sources(&f.env, &manifest).is_err(),
        "the unused catalog is still reachable, so this proves nothing"
    );
    assert!(sync_declared_sources(&f.env, &manifest).is_empty());
    assert_eq!(cache_head(&f.env, REPO, None).unwrap().len(), 7);
}

/// A catalog out of reach must not strand the items that came from every
/// other catalog: the reachable ones still resolve and the failure is
/// reported rather than thrown.
#[test]
fn a_refresh_reports_an_unreachable_catalog_and_resolves_the_rest() {
    let f = fixture();
    let mut manifest = manifest::seed(&[]);
    for (name, repo) in [("cat", REPO), ("gone", "owner/gone")] {
        manifest.sources.insert(
            name.to_owned(),
            manifest::SourceDecl {
                repo: Some(repo.to_owned()),
                path: None,
                rev: None,
                enabled: true,
            },
        );
        manifest.skills.insert(
            format!("from-{name}"),
            manifest::ItemDecl::from_source(name),
        );
    }
    manifest.sources.remove(manifest::DEFAULT_SOURCE_NAME);

    let notes = sync_declared_sources(&f.env, &manifest);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("gone"), "{notes:?}");
    // The reachable catalog resolved despite the other one failing first.
    assert_eq!(cache_head(&f.env, REPO, None).unwrap().len(), 7);
}

/// The default repository moved (vanillagreencom/vstack →
/// vanillagreencom/kendex): what the old spelling fetched serves the new
/// one with no network at all. Without the store adopting the old key's
/// directories, the moved default would read as never fetched and an
/// offline scope would go Pending.
#[test]
fn the_moved_default_serves_its_pre_move_cache_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("base").join(manifest::LEGACY_SOURCE_REPO);
    fs::create_dir_all(upstream.join("skills/gh")).unwrap();
    fs::write(
        upstream.join("skills/gh/SKILL.md"),
        "---\nname: gh\n---\nBody.\n",
    )
    .unwrap();
    super::git(&upstream, &["init", "--quiet", "-b", "main"]);
    let commit = super::commit(&upstream, "one");
    let base = format!("file://{}", tmp.path().join("base").display());
    let env = crate::env::Env::fake(tmp.path(), crate::env::FakeOs::Linux)
        .with_var("KENDEX_GIT_BASE", &base);

    // Seed the cache as a pre-move version left it: under the old
    // spelling's own key. Today's resolvers derive one key for both
    // spellings, so they could never write this state themselves.
    let url = crate::remote::clone_url(&env, manifest::LEGACY_SOURCE_REPO);
    let old_key = crate::remote::store::repo_key(&url);
    let mirror = crate::remote::store::mirror_dir(&env, &old_key);
    crate::remote::store::ensure_mirror(&mirror, &url).unwrap();
    crate::remote::store::publish(&env, &old_key, &mirror, &commit).unwrap();
    fs::remove_dir_all(tmp.path().join("base")).unwrap();

    let resolution = crate::remote::cached(&env, manifest::DEFAULT_SOURCE_REPO, None)
        .unwrap()
        .expect("the old spelling's cache serves the moved default");
    assert_eq!(resolution.commit, commit);
    assert!(resolution.root.join("skills/gh/SKILL.md").is_file());

    // The unmigrated spelling reads the very same cache entry: both
    // spellings are one key, so adoption can never strand one of them.
    let old = crate::remote::cached(&env, manifest::LEGACY_SOURCE_REPO, None)
        .unwrap()
        .expect("the old spelling resolves after adoption");
    assert_eq!(old.commit, commit);
    assert_eq!(old.root, resolution.root);
}
