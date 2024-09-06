import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { FooterComponent } from '../../../layout/footer/footer.component';

// Content
import { content } from '../../content/content';
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { Language } from '../../../services/language.types';
import { CommonModule } from '@angular/common';
import { TitleService } from '../../../services/navTitle.service';

@Component({
  selector: 'docs-t01-into-getting-starter',
  standalone: true,
  imports: [
    RouterOutlet,
    FooterComponent,
    CommonModule
  ],
  templateUrl: './into-getting-starter.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T01_IntoComponent implements OnInit, AfterViewInit {

  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService,
    private TitleService: TitleService
  ) { }

  // For inner content
  write: any;

  // Language
  currentLanguage: Language = 'ES';

  ngOnInit(): void {
    // Initial content
    this.write = content[this.currentLanguage].title_01.page_01;

    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_01.page_01;
        this.currentLanguage = language;
      }
    });
    // Set TitleValue Service
    this.TitleService.setTitle('introduction');
  }

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  // Hash Sections
  hovered = false;

  showHash(event: Event) {
    this.hovered = true;
  }
  
  hideHash(event: Event) {
    this.hovered = false;
  }

    // service Hash Sections
    @ViewChildren('section') sections!: QueryList<ElementRef>;
    idActive: string = '';
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
  });
}
}