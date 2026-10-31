# Checked bundle publication (Engine adoption prerequisite)

The pipeline must return an error, not a successful bundle tuple, when required
persistence, artifact validation, recovery-marker cleanup or durability fails.
This is a persistence bug fix, not a new output format or an Engine receipt.

## Caller ordering

The final barrier belongs **after** all optional whiteboard mutations, including
rollback/fallback to the original screen bundle. A proof of the base bundle is
not a proof of a later rewritten contact sheet, manifest or summary.

The GUI whiteboard integration is owned separately. Its screen and phone callers
must hold publication coordination across the production finalizer and
`whiteboard::finish_handoff`, then validate the returned bundle and summary before
allowing `receive_capture_result`, Saved, clipboard or upload. A failed optional
whiteboard may fall back only if the restored base passes the same barrier.
A failed barrier must stay an error through the existing result type.

The Engine caller will consume the final exact manifest bytes and bundle path,
with its own request identity, using the existing `FinalizerSuccess::validate`
and `Session::finish`. This layer does not mint requests, receipts or Engine Saved.
Native Mac/Linux proof and coordinator containment remain separate acceptance
steps. No native capture, VM, install, deploy or live config change is authorized
by the offline tests.
