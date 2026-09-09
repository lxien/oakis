
(function () {
  var MAX_IMAGE_BYTES = 5 * 1024 * 1024;
  var ACCEPT_RE = /^image\/(png|jpeg|jpg|webp|gif)$/i;

  function ready(fn) {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", fn);
    } else {
      fn();
    }
  }

  function orphanEditorFileInputs(root) {
    var scope = root || document;
    scope.querySelectorAll(".EasyMDEContainer input[type='file']").forEach(function (input) {
      input.removeAttribute("name");
      input.setAttribute("form", "oakis-editor-nofile");
      try {
        input.value = "";
      } catch (e) {}
    });
  }

  function uploadImageFile(file) {
    var body = new FormData();
    body.append("file", file, file.name || "paste.png");
    var headers = { Accept: "application/json" };
    if (window.OakisCsrf && window.OakisCsrf.token) {
      var token = window.OakisCsrf.token();
      if (token) headers[window.OakisCsrf.headerName || "X-CSRF-Token"] = token;
    }
    return fetch("/admin/upload", {
      method: "POST",
      body: body,
      credentials: "same-origin",
      headers: headers,
    }).then(function (res) {
      return res.text().then(function (text) {
        var data = null;
        try {
          data = text ? JSON.parse(text) : null;
        } catch (e) {
          data = null;
        }
        if (!res.ok) {
          throw new Error(
            (data && data.error) ||
              (res.status === 403
                ? "请求无效或已过期，请刷新页面后重试"
                : "上传失败")
          );
        }
        if (!data || !data.url) throw new Error("上传失败");
        return data.url;
      });
    });
  }

  function collectClipboardImages(clipboardData) {
    var out = [];
    if (!clipboardData) return out;

    var items = clipboardData.items;
    if (items && items.length) {
      for (var i = 0; i < items.length; i++) {
        var item = items[i];
        if (item.kind !== "file" || !ACCEPT_RE.test(item.type || "")) continue;
        var file = item.getAsFile();
        if (file) out.push(file);
      }
    }

    if (!out.length && clipboardData.files && clipboardData.files.length) {
      for (var j = 0; j < clipboardData.files.length; j++) {
        var f = clipboardData.files[j];
        if (f && ACCEPT_RE.test(f.type || "")) out.push(f);
      }
    }
    return out;
  }

  function insertMarkdownImage(cm, url, name) {
    var alt = String(name || "image").replace(/[\[\]]/g, "");
    var doc = cm.getDoc();
    var cursor = doc.getCursor();
    var md = "![" + alt + "](" + url + ")";
    doc.replaceRange(md, cursor);
    cm.focus();
  }

  function insertMarkdownLink(cm, url, name) {
    var text = String(name || "附件").replace(/[\[\]]/g, "");
    var doc = cm.getDoc();
    var cursor = doc.getCursor();
    doc.replaceRange("[" + text + "](" + url + ")", cursor);
    cm.focus();
  }

  function isImageItem(item) {
    if (!item) return false;
    if (item.mime && String(item.mime).toLowerCase().indexOf("image/") === 0) return true;
    return /\.(png|jpe?g|webp|gif)(\?|$)/i.test(String(item.url || ""));
  }

  function insertPickedMedia(cm, item) {
    if (!item || !item.url) return;
    if (isImageItem(item)) {
      insertMarkdownImage(cm, item.url, item.name);
    } else {
      insertMarkdownLink(cm, item.url, item.name);
    }
  }

  ready(function () {
    var el = document.getElementById("content-md");
    if (!el || typeof EasyMDE === "undefined") return;

    var editor = new EasyMDE({
      element: el,
      spellChecker: false,
      autofocus: false,
      forceSync: true,

      sideBySideFullscreen: false,
      status: ["lines", "words", "cursor", "upload-image"],
      minHeight: "420px",
      placeholder: "Markdown",
      indentWithTabs: false,
      tabSize: 2,
      unorderedListStyle: "-",
      renderingConfig: {
        singleLineBreaks: false,
        codeSyntaxHighlighting: false,
      },
      toolbar: [
        "bold",
        "italic",
        "heading",
        "|",
        "quote",
        "unordered-list",
        "ordered-list",
        "|",
        "link",
        {
          name: "media-library",
          className: "fa fa-folder-open",
          title: "媒体 / 附件",
          action: function (ed) {
            if (!window.OakisMediaPicker) return;
            OakisMediaPicker.open({
              kind: "all",
              onSelect: function (item) {
                insertPickedMedia(ed.codemirror, item);
              },
            });
          },
        },
        "table",
        "code",
        "|",
        "preview",
        "side-by-side",
        "fullscreen",
        "|",
        "guide",
      ],
      uploadImage: true,
      imageMaxSize: MAX_IMAGE_BYTES,
      imageAccept: "image/png,image/jpeg,image/webp,image/gif",
      imageUploadFunction: function (file, onSuccess, onError) {
        uploadImageFile(file)
          .then(function (url) {
            orphanEditorFileInputs();
            onSuccess(url);
          })
          .catch(function (err) {
            orphanEditorFileInputs();
            onError(err.message || "上传失败");
          });
      },
    });

    orphanEditorFileInputs();

    var editorRoot = editor.codemirror.getWrapperElement().closest(".EasyMDEContainer");
    if (editorRoot) {
      editorRoot.querySelectorAll(".editor-preview, .editor-preview-side").forEach(function (pane) {
        pane.classList.add("prose");
      });
    }

    editor.codemirror.getWrapperElement().addEventListener(
      "paste",
      function (event) {
        var files = collectClipboardImages(event.clipboardData);
        if (!files.length) return;

        event.preventDefault();
        event.stopImmediatePropagation();

        var cm = editor.codemirror;
        files.forEach(function (file) {
          if (file.size > MAX_IMAGE_BYTES) {
            if (typeof layui !== "undefined") {
              layui.use("layer", function () {
                layui.layer.msg("图片过大（最大 5MB）");
              });
            }
            return;
          }
          uploadImageFile(file)
            .then(function (url) {
              orphanEditorFileInputs();

              insertMarkdownImage(cm, url, "image");
            })
            .catch(function (err) {
              orphanEditorFileInputs();
              if (typeof layui !== "undefined") {
                layui.use("layer", function () {
                  layui.layer.msg(err.message || "上传失败");
                });
              }
            });
        });
      },
      true
    );

    var form = el.closest("form");
    if (form) {
      form.addEventListener("submit", function () {
        editor.codemirror.save();
        orphanEditorFileInputs(form);
      });
    }
  });
})();
