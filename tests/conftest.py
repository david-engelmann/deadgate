"""Synthetic input writers shared by the suite.

Every fixture written through these helpers is invented for this
repo and stands in the real form of the input it imitates.
"""

import json

import pytest


def make_record(
    name,
    conclusion="success",
    status="completed",
    started="2026-10-14T09:00:00Z",
    completed="2026-10-14T09:04:00Z",
    workflow_name="ci",
):
    return {
        "id": 1,
        "run_id": 1,
        "workflow_name": workflow_name,
        "name": name,
        "status": status,
        "conclusion": conclusion,
        "started_at": started,
        "completed_at": completed,
    }


class Inputs:
    def __init__(self, root):
        self.root = root

    def protection(self, contexts, style="checks"):
        if style == "checks":
            payload = {
                "required_status_checks": {
                    "strict": True,
                    "contexts": list(contexts),
                    "checks": [
                        {"context": context, "app_id": 15368} for context in contexts
                    ],
                }
            }
        elif style == "legacy":
            payload = {"required_status_checks": {"contexts": list(contexts)}}
        else:
            payload = style
        path = self.root / "protection.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        return str(path)

    def workflows(self, files):
        folder = self.root / "workflows"
        folder.mkdir(exist_ok=True)
        for name, text in files.items():
            (folder / name).write_text(text, encoding="utf-8")
        return str(folder)

    def history(self, records):
        path = self.root / "history.json"
        payload = {"total_count": len(records), "jobs": records}
        path.write_text(json.dumps(payload), encoding="utf-8")
        return str(path)


@pytest.fixture
def inputs(tmp_path):
    return Inputs(tmp_path)


@pytest.fixture
def rec():
    return make_record
