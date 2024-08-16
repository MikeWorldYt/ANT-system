import { CommonModule } from '@angular/common';
import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';

// ▲ SERVICES ▲
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LanguageService } from '../../../services/lenguaje.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';

@Component({
  selector: 'doc-t02-mod-03-nomenclature',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './mod-03-nomenclature.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod03_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ███ Fill Content (inner) ███ 
  ngOnInit(): void {
    // Initial content 
    this.write = content[this.currentLanguage].title_02.module_03;
    // Suscribe to Language Service
    this.languageService.language$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.module_03;
        this.currentLanguage = language;
      }
    });
  }

  // ▬▬▬ Language
  currentLanguage: Language = 'ES';

  // ███ Language Controller ███
  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  switchLanguage(language: Language) {
    this.languageService.setLanguage(language);
  }

  // ▬▬▬ Hash Sections
  hovered: boolean = false;

  // ███ Hash Sections ███
  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ▬▬▬ Intersection Section <section>
  idActive: string = '';

  // ▲ Hash Sections Service
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  ngAfterViewInit() {
    // Intersection Section
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
  }

}
