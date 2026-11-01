package dev.horizon.android.launcher

import org.json.JSONArray
import org.json.JSONObject

private fun JSONObject.text(key: String): String? = if (isNull(key)) null else optString(key)
private fun <T> JSONArray?.list(map: (JSONObject) -> T): List<T> =
    if (this == null) emptyList() else List(length()) { map(getJSONObject(it)) }

data class Install(
    val installed: Boolean,
    val installing: Boolean,
    val stage: String,
    val stageIndex: Int,
    val stages: List<String>,
    val overall: Float,
    val detail: String,
)

data class LoginCode(val userCode: String, val uri: String)

data class Account(
    val status: String,
    val signedIn: Boolean,
    val profile: String?,
    val pending: Boolean,
    val message: String,
    val code: LoginCode?,
    val needsReview: Boolean,
)

data class Config(
    val name: String,
    val server: String,
    val profile: Int,
    val profileName: String,
    val renderer: String,
    val renderScale: Int,
    val input: String,
    val modSource: String,
    val shaderSource: String,
)

data class SettingRow(
    val id: String,
    val title: String,
    val value: String,
    val kind: String,
    val on: Boolean,
)

data class SettingGroup(val name: String, val rows: List<SettingRow>)

data class Project(
    val index: Int,
    val title: String,
    val summary: String,
    val author: String,
    val downloads: Long,
    val state: String?,
    val note: String?,
    val button: String,
)

data class CatalogState(
    val page: String,
    val query: String,
    val results: List<Project>,
    val offset: Int,
    val busy: Boolean,
)

data class LocalItem(
    val index: Int,
    val kind: String,
    val title: String,
    val file: String,
    val enabled: Boolean,
    val toggle: Boolean,
)

data class Conflict(val name: String, val keepBoth: Boolean)

data class Documents(val working: Boolean, val text: String?, val conflict: Conflict?)

data class Snapshot(
    val version: String,
    val status: String,
    val error: String?,
    val install: Install,
    val account: Account,
    val config: Config,
    val settings: List<SettingGroup>,
    val catalog: CatalogState,
    val local: List<LocalItem>,
    val documents: Documents,
) {
    companion object {
        fun parse(text: String): Snapshot? {
            val root = runCatching { JSONObject(text) }.getOrNull() ?: return null
            val install = root.getJSONObject("install")
            val account = root.getJSONObject("account")
            val config = root.getJSONObject("config")
            val catalog = root.getJSONObject("catalog")
            val documents = root.getJSONObject("documents")
            return Snapshot(
                version = root.optString("version"),
                status = root.optString("status"),
                error = root.text("error"),
                install = Install(
                    installed = install.optBoolean("installed"),
                    installing = install.optBoolean("installing"),
                    stage = install.optString("stage"),
                    stageIndex = install.optInt("stage_index"),
                    stages = install.optJSONArray("stages").let { a ->
                        if (a == null) emptyList() else List(a.length()) { a.getString(it) }
                    },
                    overall = install.optDouble("overall", 0.0).toFloat(),
                    detail = install.optString("detail"),
                ),
                account = Account(
                    status = account.optString("status"),
                    signedIn = account.optBoolean("signed_in"),
                    profile = account.text("profile"),
                    pending = account.optBoolean("pending"),
                    message = account.optString("message"),
                    code = account.optJSONObject("code")?.let {
                        LoginCode(it.optString("user_code"), it.optString("uri"))
                    },
                    needsReview = account.optBoolean("needs_review"),
                ),
                config = Config(
                    name = config.optString("name"),
                    server = config.optString("server"),
                    profile = config.optInt("profile"),
                    profileName = config.optString("profile_name"),
                    renderer = config.optString("renderer"),
                    renderScale = config.optInt("render_scale"),
                    input = config.optString("input"),
                    modSource = config.optString("mod_source"),
                    shaderSource = config.optString("shader_source"),
                ),
                settings = root.optJSONArray("settings").list { group ->
                    SettingGroup(
                        group.optString("group"),
                        group.optJSONArray("rows").list {
                            SettingRow(
                                it.optString("id"),
                                it.optString("title"),
                                it.optString("value"),
                                it.optString("kind"),
                                it.optBoolean("on"),
                            )
                        },
                    )
                },
                catalog = CatalogState(
                    page = catalog.optString("page"),
                    query = catalog.optString("query"),
                    results = catalog.optJSONArray("results").list {
                        Project(
                            index = it.optInt("index"),
                            title = it.optString("title"),
                            summary = it.optString("summary"),
                            author = it.optString("author"),
                            downloads = it.optLong("downloads"),
                            state = it.text("state"),
                            note = it.text("note"),
                            button = it.optString("button"),
                        )
                    },
                    offset = catalog.optInt("offset"),
                    busy = catalog.optBoolean("busy"),
                ),
                local = root.optJSONArray("local").list {
                    LocalItem(
                        index = it.optInt("index"),
                        kind = it.optString("kind"),
                        title = it.optString("title"),
                        file = it.optString("file"),
                        enabled = it.optBoolean("enabled"),
                        toggle = it.optBoolean("toggle"),
                    )
                },
                documents = Documents(
                    working = documents.optBoolean("working"),
                    text = documents.text("text"),
                    conflict = documents.optJSONObject("conflict")?.let {
                        Conflict(it.optString("name"), it.optBoolean("keep_both"))
                    },
                ),
            )
        }
    }
}

data class Choice(val label: String, val detail: String, val value: String, val selected: Boolean)

data class Options(val title: String, val help: String, val choices: List<Choice>)

/** The result of [dev.horizon.android.NativeBridge.call]. */
data class Reply(val ok: Any?, val error: String?) {
    companion object {
        fun parse(text: String?): Reply {
            val root = text?.let { runCatching { JSONObject(it) }.getOrNull() }
                ?: return Reply(null, "No reply from the launcher")
            return Reply(root.opt("ok"), root.text("error"))
        }
    }

    fun options(): Options? {
        val o = ok as? JSONObject ?: return null
        return Options(
            o.optString("title"),
            o.optString("help"),
            o.optJSONArray("choices").list {
                Choice(it.optString("label"), it.optString("detail"), it.optString("value"), it.optBoolean("selected"))
            },
        )
    }
}

/** One entry of the profile manager: a folder with its own worlds, settings and add-ons. */
data class Profile(
    val id: Int,
    val name: String,
    val active: Boolean,
    val worlds: Int,
    val mods: Int,
    val packs: Int,
    val shaders: Int,
) {
    companion object {
        fun load(): List<Profile> = (act("profiles").ok as? JSONArray).list {
            Profile(
                it.optInt("id"),
                it.optString("name"),
                it.optBoolean("active"),
                it.optInt("worlds"),
                it.optInt("mods"),
                it.optInt("packs"),
                it.optInt("shaders"),
            )
        }
    }
}
