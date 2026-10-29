"""Linux-checkable packaging contract; native encoder behavior still needs a Mac."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MacWriterWiringTest(unittest.TestCase):
    def test_swift_code_stays_resident_after_destroy_and_create_failure(self):
        source = (ROOT / "src/capture/realtime_writer.rs").read_text()
        self.assertNotIn('libc::dlclose(', source)

    def test_display_region_keep_legacy_stop_without_candidate_wait(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        branch = source.split('if !super::uses_realtime_writer(sampler_target.mode) {', 1)[1]
        branch = branch.split('return video_path;', 1)[0]
        self.assertIn('stream.stop_capture()', branch)
        self.assertIn('stream.remove_recording_output(output)', branch)
        self.assertIn('stream_unsafe_video.store(false', branch)
        self.assertNotIn('bounded_candidate', branch)
        self.assertIn('drop_since(prompts.clear_until_ms)', branch)

    def test_writer_is_created_only_for_window_mode(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        setup = source.split('fn setup_sck_capture(', 1)[1]
        self.assertLess(setup.index('if !super::uses_realtime_writer(target.mode)'),
                        setup.index('Writer::create('))
        self.assertIn('setup_legacy_sck_capture(', setup)

    def test_original_certification_is_independent_of_native_timeout(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        start = source.index('let result = super::bounded_candidate(')
        end = source.index('stream_unsafe_video.store(false', start)
        self.assertNotIn('original_privacy_safe =', source[start:end])
        self.assertIn('if original_privacy_safe && video_path.is_some()', source[start:end])

    def test_stop_geometry_is_window_only(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        geometry = source.split('fn stop_target_geometry(', 1)[1].split('/// Re-resolve', 1)[0]
        self.assertIn('super::same_window_owner(', geometry)
        self.assertNotIn('crate::screens::displays()', geometry)

    def test_window_filter_requires_pinned_owner_and_has_no_display_fallback(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        filters = source.split('fn build_sck_filter_and_config_for_owner(', 1)[1]
        window = filters.split('} else if let Some(display)', 1)[0]
        self.assertIn('super::same_window_owner(', window)
        self.assertIn('SCContentFilter::create().with_window(&window)', window)
        self.assertIn('target.mode == CaptureMode::Window', window)
        self.assertIn('None', window.split('target.mode == CaptureMode::Window', 1)[1])

    def test_loader_uses_one_process_lifetime_reference(self):
        source = (ROOT / "src/capture/realtime_writer.rs").read_text()
        self.assertIn('OnceLock<Result<WriterApi, String>>', source)
        self.assertEqual(source.count('libc::dlopen('), 1)

    def test_native_writer_holds_retention_lease(self):
        source = (ROOT / "src/capture/realtime_writer.rs").read_text()
        self.assertIn('_media_lease: crate::retention::MediaLease', source)
        self.assertLess(source.index('crate::retention::candidate_media_lease('),
                        source.index('let handle = unsafe { create('))

    def test_dedicated_dylib_is_not_part_of_helper_executable(self):
        build = (ROOT / "build.rs").read_text()
        self.assertIn('path.file_name().is_some_and(|name| name != "RealtimeWriter.swift")', build)
        self.assertIn('compile_realtime_writer();', build)
        self.assertIn('"-emit-library"', build)
        self.assertIn('"libapparent_realtime_writer.dylib"', build)

    def test_all_mac_bundle_paths_copy_and_sign_writer(self):
        for name in ("install-app.sh", "build-mac-preview.sh", "release-app.sh"):
            with self.subTest(name=name):
                script = (ROOT / "scripts" / name).read_text()
                self.assertIn('libapparent_realtime_writer.dylib', script)
                self.assertTrue('Contents/Frameworks' in script or
                                'FRAMEWORKS_DIR="${CONTENTS_DIR}/Frameworks"' in script)
                self.assertIn('codesign', script)
        remote = (ROOT / "scripts/install-app-remote.sh").read_text()
        self.assertIn('ditto "${STAGE}" "${APP}"', remote)

    def test_abi_has_bounded_failure_and_single_adaptor(self):
        source = (ROOT / "helper/RealtimeWriter.swift").read_text()
        for symbol in ("apparent_writer_create_v1", "apparent_writer_enqueue_sample_v1",
                       "apparent_writer_enqueue_final_rgba_v1", "apparent_writer_finish_v1",
                       "apparent_writer_destroy_v1"):
            self.assertIn('@_cdecl("' + symbol + '")', source)
        self.assertIn('AVAssetWriterInputPixelBufferAdaptor(', source)
        self.assertIn('maxPendingSamples', source)
        self.assertIn('Unmanaged.passUnretained(sample).retain()', source)

    def test_native_candidate_waits_for_a_second_privacy_check_before_publish(self):
        source = (ROOT / "src/capture/macos.rs").read_text()
        start = source.index('let result = super::bounded_candidate(')
        postwait = source.index('prompts.checked = None;', start)
        publish = source.index('super::publish_validated_movie(', start)
        self.assertLess(start, postwait)
        self.assertLess(postwait, publish)
        self.assertIn('stop_target_geometry(&sampler_target) != stop_geometry',
                      source[postwait:publish])
        self.assertIn('prompts.check(started_at) || prompts.showing()',
                      source[postwait:publish])

    def test_native_destroy_never_joins_the_encoder_queue(self):
        source = (ROOT / "helper/RealtimeWriter.swift").read_text()
        destroy = source.split('public func apparent_writer_destroy_v1(', 1)[1].split('// Read both', 1)[0]
        self.assertIn('session.cancelIfOpen()', destroy)
        self.assertNotIn('session.finish()', destroy)


if __name__ == "__main__":
    unittest.main()
