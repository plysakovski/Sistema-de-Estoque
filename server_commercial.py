import http.server
import json
import os
import base64
import glob

PORT = 8000
DIR  = os.path.dirname(os.path.abspath(__file__))
ALLOWED_SAVE = ['unidades.xlsx', 'categorias.xlsx', 'categorias.ods']

def find_cat_file():
    for name in ['categorias.xlsx','categorias.ods']:
        if os.path.exists(os.path.join(DIR, name)):
            return name, os.path.join(DIR, name)
    for f in os.listdir(DIR):
        fl = f.lower()
        if fl.endswith(('.ods','.xlsx')) and f.lower() not in ('unidades.xlsx',):
            if 'categoria' in fl or 'estoque' in fl or 'inventario' in fl:
                return f, os.path.join(DIR, f)
    return None, None

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIR, **kwargs)

    def do_GET(self):
        path = self.path.split('?')[0].lstrip('/')
        if path == 'check':
            files = sorted(os.listdir(DIR))
            sheets = [f for f in files if f.endswith(('.xlsx','.ods'))]
            cat_name, _ = find_cat_file()
            result = {
                'dir': DIR,
                'spreadsheets': sheets,
                'unidades_found': os.path.exists(os.path.join(DIR,'unidades.xlsx')),
                'categorias_found': cat_name is not None,
                'categorias_detected_as': cat_name or 'NOT FOUND',
            }
            body = json.dumps(result, ensure_ascii=False).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type','application/json; charset=utf-8')
            self.send_header('Content-Length', len(body))
            self.end_headers()
            self.wfile.write(body)
            return

        if path.lower().startswith('categorias'):
            cat_name, cat_path = find_cat_file()
            if cat_name and cat_path:
                try:
                    with open(cat_path,'rb') as f:
                        data = f.read()
                    ctype = 'application/vnd.oasis.opendocument.spreadsheet' if cat_path.endswith('.ods') else \
                            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
                    self.send_response(200)
                    self.send_header('Content-Type', ctype)
                    self.send_header('Content-Length', len(data))
                    self.end_headers()
                    self.wfile.write(data)
                    print('  [LIDO] %s (%d KB)' % (cat_name, len(data)//1024))
                    return
                except Exception as e:
                    print('  [ERRO ao ler %s]: %s' % (cat_name, e))
        super().do_GET()

    def do_POST(self):
        if self.path == '/save':
            try:
                length = int(self.headers.get('Content-Length', 0))
                body   = self.rfile.read(length)
                data   = json.loads(body)
                b64    = data.get('data', '')
                fname  = os.path.basename(data.get('filename', ''))
                cat_name, _ = find_cat_file()
                allowed = ALLOWED_SAVE[:]
                if cat_name and cat_name not in allowed:
                    allowed.append(cat_name)
                if fname not in allowed:
                    self._respond(403, json.dumps({"error":"not allowed","allowed":allowed}))
                    return
                filepath = os.path.join(DIR, fname)
                with open(filepath, 'wb') as f:
                    f.write(base64.b64decode(b64))
                print('  [SALVO] %s (%d KB)' % (fname, os.path.getsize(filepath)//1024))
                self._respond(200, '{"ok":true}')
            except Exception as e:
                print('  [ERRO SAVE] %s' % e)
                self._respond(500, json.dumps({"error": str(e)}))
        else:
            self._respond(404, '{"error":"not found"}')

    def _respond(self, code, body):
        b = body.encode('utf-8') if isinstance(body, str) else body
        self.send_response(code)
        self.send_header('Content-Type','application/json')
        self.send_header('Content-Length', len(b))
        self.send_header('Access-Control-Allow-Origin','*')
        self.end_headers()
        self.wfile.write(b)

    def log_message(self, fmt, *args):
        status = str(args[1]) if len(args)>1 else ''
        path   = str(args[0]) if args else ''
        if status in ('200','304'): return
        if status == '404' and ('favicon' in path or 'categorias' in path.lower()): return
        super().log_message(fmt, *args)

    def log_error(self, fmt, *args):
        msg = (fmt % args) if args else str(fmt)
        if 'favicon' in msg.lower() or 'categorias' in msg.lower(): return
        super().log_error(fmt, *args)

os.chdir(DIR)
print()
print('  ============================================')
print('   StockManager Pro - Servidor rodando')
print('  ============================================')
print('  Acesse : http://localhost:%d' % PORT)
print('  Pasta  : %s' % DIR)
print()
inv_ok  = os.path.exists(os.path.join(DIR,'unidades.xlsx'))
cat_nm, _ = find_cat_file()
print('  unidades.xlsx : %s' % ('ENCONTRADO' if inv_ok else 'NAO ENCONTRADO'))
print('  categorias    : %s' % (cat_nm if cat_nm else 'NAO ENCONTRADO'))
print()
print('  Diagnostico: http://localhost:%d/check' % PORT)
print('  Para encerrar: feche esta janela')
print('  ============================================')
print()
httpd = http.server.HTTPServer(('localhost', PORT), Handler)
httpd.serve_forever()
