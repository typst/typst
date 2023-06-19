#import "template.typ": case

/* Calibrage */
#place(top+left, dx : -60pt, dy : -60pt, square(size: 10pt, fill : black))
#place(bottom+right, dx : 60pt, dy : 60pt, square(size: 10pt, fill : black))

/* Grille numéro étudiant 
La fonction id_etudiant génère une grille de 8 colonnes et de 10 lignes */

#let id_etudiant() = [
  #let colonne_cases() =[
    #let i = 0
    #while i < 10 {
      square(size: 3pt, outset: 4pt)
      i=i+1
    }
    ]

  #let colonne_chiffre() = [
    #let i = 0
    #while i < 10 {
      [#i]
      linebreak() 
      v(-4.5pt)
      i=i+1
    }
  ]
  #box()[
      #columns(9, gutter: -300pt)[
      #colonne_chiffre()
      #colbreak()
      #let n=0
      #while n < 8{
        colonne_cases()
        colbreak()
        n=n+1
      } 
    ]
  ]
]


/* Logo
- Importer un logo sous l'appelation "Logo.png"
*/

#let logo(taille)=[
  #if taille == "grand"{
    image("Logo.png", width: 100%)
  }
  else if taille == "moyen grand"{
    image("Logo.png", width: 75%)
  }
  else if taille == "moyen"{
    image("Logo.png", width: 50%)
  }
  else if taille == "moyen petit"{
    image("Logo.png", width: 25%)
  }
  else if taille == "petit"{
    image("Logo.png", width: 0%)
  }
  
]
/* Une fonction pour tout
- type_q : type de la question (multiple true false, 1 parm N...)
- type_a : type d'affichage : afficher les réponses en sautant une ligne = "saut"
                              à la suite = "suite"
- q : question
*/

#let affichage_question(type_q,type_a, q)= {
  /* type d'affichage */
  let affichage_defaut(type_a, q) = {
    if type_a == "a_la_suite"{
     columns( q.numberOfAnswers, gutter : -200pt)[
        #let nOA = q.numberOfAnswers
        #align(center)[
          /* Pour chaque réponse */
          #for a in q.answers {
            block()[#v(5pt)#a.answerLabel
            #align(center)[#box(case(a.answerId, size : 10pt))]
            ]
            if nOA > 1 {
              colbreak()
            }
            nOA=nOA - 1
        }
        #v(20pt)//espacement après exo
        ]]
    } else{
      for a in q.answers [
          #box(case(a.answerId, size : 10pt)) #h(8pt) #a.answerLabel
        #v(2pt)
      ]
       v(10pt)//espacement après exo
    }
  }
  let affichage_TF(type_a, q) = {
    {
      for a in q.answers [
          #box(width: 25pt, columns([#case(a.answerId + "T", size : 10pt)#colbreak() #case(a.answerId + "F", size : 10pt)])) #h(8pt) #a.answerLabel
        #v(2pt) // OMG !
      ]
       v(10pt)//espacement après exo
    }
  }
  
  /* type de question */
  if type_q == "MCQ"{
    [_plusieurs réponses peuvent être justes_
  
  ]
    affichage_defaut(type_a, q)
  }
  else if type_q == "1parmiN"{
    [_une seule réponse est juste_
  
  ]
    affichage_defaut(type_a, q)
  }
   else if type_q == "multipleTF"{
    [_une seule réponse par question_
  
  ]
    affichage_TF(type_a, q)
  }

}

/* Presentation() affiche les premiers éléments indispendables pour identifier une copie */

#let presentation(exam)=[
  #place(top+left, dx : 0pt, dy : -10pt)[#logo("moyen petit")]
  #place(top+right, dx : 0pt, dy : -10pt)[Numéro Étudiant #v(0pt) #id_etudiant()]
  #place(top+left, dx : 10pt, dy : 1.8cm)[
  #rect(width: 7cm, height: 3cm, inset: 11pt)[Nom : #v(15pt) Prenom :]]
  #v(7cm)
  #set heading(numbering:"1.") //Numérotation
  // Espace identifiant étudiant
  #underline[*#align(center, text(size : 15pt, exam.title))*] //Titre
  #place(top+right, dx : 50pt, dy : -50pt, [#rect[#exam.examId]]) //Identifiant
]

#let forecast(exam) =[    
  #presentation(exam)
  /* Parcourir chaque exercice */
  #for ex in exam.exercises {
    block()
      [#align(center)[#rect[= #upper[#text(size : 13pt, "Exercice")]]]//Titre de l'exercice
      #align(center)[#ex.eSText] //Énoncé de l'exercice
      #v(15pt)
      /* Parcourir chaque question */
      #for q in ex.questions {
          [== #q.qStatement]
          affichage_question(q.questionType, "saut", q)
      }
      ] 
  }
]

#let text = "exemple"

/*Force régénération*/
= Real <test>
#locate(loc => {
  let elems = query(<test>, loc)
  let count = elems.len()
  count * [#pagebreak()]
})

#forecast(json(text + ".json"))


/*On observe que nos positions ne sont pas affectées par les régénérations!*/
