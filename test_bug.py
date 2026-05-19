import subprocess
import os
import sys

TEST_CASES = {
    "bug_8218_pagebreak": {
        "description": "Contatore dopo pagebreak",
        "code": """
        #let c = counter("test")
        #c.update(0)
        #context {
          let n = c.get()
          c.step()
          pagebreak()
          [EXPECTED_START:#n:EXPECTED_END]
        }
        """,
        "expected": "0"
    }
}

def run_test(test_name, test_data):
    print(f"Avvio test: {test_name}")
    temp_typ = f"temp_{test_name}.typ"
    temp_pdf = f"temp_{test_name}.pdf"
    with open(temp_typ, "w", encoding="utf-8") as f:
        f.write(test_data["code"])
    try:
        result = subprocess.run(["cargo", "run", "--bin", "typst", "--", "compile", temp_typ, temp_pdf], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if result.returncode != 0:
            print(f"Errore esecuzione:\n{result.stderr}")
            return False
        if not os.path.exists(temp_pdf):
            print("Errore: output PDF assente")
            return False
        with open(temp_pdf, "rb") as f:
            pdf_content = f.read().decode("latin-1", errors="ignore")
        if f"EXPECTED_START:{test_data["expected"]}:EXPECTED_END" in pdf_content:
            print("[ TEST SUPERATO ] La tua modifica funziona! Il bug e stato risolto con successo!")
            return True
        else:
            print("[ TEST FALLITO ] Il bug e ancora attivo o la logica non ha funzionato.")
            return False
    finally:
        for temp_file in [temp_typ, temp_pdf]:
            if os.path.exists(temp_file):
                os.remove(temp_file)

if __name__ == "__main__":
    for name, data in TEST_CASES.items():
        run_test(name, data)
    input("Premere INVIO per uscire...")
