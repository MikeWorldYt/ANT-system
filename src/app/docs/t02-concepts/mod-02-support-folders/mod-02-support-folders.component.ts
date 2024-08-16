import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { CommonModule } from '@angular/common';

// ▲ SERVICES ▲
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LanguageService } from '../../../services/lenguaje.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';

// Testing
import { HeaderT02Component } from '../header-t02/header-t02.component';

@Component({
  selector: 'doc-t02-mod-02-support-folders',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule
  ],
  templateUrl: './mod-02-support-folders.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod02_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }
  // ▬▬▬ For inner content
  write: any;

  // ████ Fill Content (inner) ███
  ngOnInit(): void {
    // Initial content
    this.write = content[this.currentLanguage].title_02.module_02;
    // Suscribe to Language Service
    this.languageService.language$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.module_02;
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

  //  ▬▬▬ Hash Sections
  hovered = false;

  // ████ Hash Sections ████
  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ▲ service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;
  
  idActive: string = '';

  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    })
  }

}
